//! OpenDeck actions for the LightCraft command line interface and its live control bridge.
//!
//! Arguments are stored as a JSON string array and sent directly to the CLI.
//! Deliberately never invoking a shell preserves paths, JSON parameters, and user input exactly.

use std::{path::PathBuf, process::Stdio, time::Duration};

use openaction::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use tokio::process::Command;

const DEFAULT_EXECUTABLE: &str = "lightcraft-cli";
const CATALOG_ARGUMENTS: &[&str] = &["commands", "--json"];
const DEFAULT_ARGUMENTS: &str = r#"["commands","--json"]"#;
const MAX_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_CATALOG_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct CommandSettings {
    /// CLI executable or absolute path. Empty uses the app's conventional CLI name.
    executable: String,
    /// JSON string array, e.g. ["exec", "timeline.play", "{}"].
    arguments: String,
    /// Optional directory in which to run the command.
    working_directory: String,
    /// A user-visible title shown before the optional command output.
    title: String,
    /// Keep stdout/stderr (trimmed) on the Stream Deck key after a successful run.
    show_output: bool,
    /// Wall-clock limit for a process. Zero uses 30 seconds; maximum is 300 seconds.
    timeout_seconds: u64,
}

impl Default for CommandSettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            arguments: DEFAULT_ARGUMENTS.to_string(),
            working_directory: String::new(),
            title: String::new(),
            show_output: false,
            timeout_seconds: 30,
        }
    }
}

impl CommandSettings {
    fn executable(&self) -> &str {
        let executable = self.executable.trim();
        if executable.is_empty() {
            DEFAULT_EXECUTABLE
        } else {
            executable
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(if self.timeout_seconds == 0 {
            30
        } else {
            self.timeout_seconds.clamp(1, 300)
        })
    }

    fn title(&self) -> Option<String> {
        (!self.title.trim().is_empty()).then(|| self.title.trim().to_string())
    }
}

#[derive(Debug)]
enum ExecutionError {
    Configuration(String),
    Process(String),
    TimedOut(u64),
    Failed { code: Option<i32>, output: String },
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(message) | Self::Process(message) => f.write_str(message),
            Self::TimedOut(seconds) => write!(f, "Timed out after {seconds} seconds"),
            Self::Failed { code, output } => {
                write!(
                    f,
                    "LightCraft CLI exited with {}",
                    code.map_or_else(|| "a signal".into(), |code| format!("code {code}"))
                )?;
                if !output.is_empty() {
                    write!(f, ": {output}")?;
                }
                Ok(())
            }
        }
    }
}

fn parse_arguments(raw: &str) -> Result<Vec<String>, ExecutionError> {
    let parsed: Value = serde_json::from_str(raw.trim()).map_err(|error| {
        ExecutionError::Configuration(format!(
            "Arguments must be a JSON array of strings: {error}"
        ))
    })?;
    let values = parsed.as_array().ok_or_else(|| {
        ExecutionError::Configuration("Arguments must be a JSON array of strings.".into())
    })?;
    if values.is_empty() {
        return Err(ExecutionError::Configuration(
            "Add at least one CLI argument, for example [\"commands\", \"--json\"].".into(),
        ));
    }
    if values.len() > MAX_ARGUMENTS {
        return Err(ExecutionError::Configuration(format!(
            "At most {MAX_ARGUMENTS} arguments are allowed."
        )));
    }
    values
        .iter()
        .map(|value| {
            let value = value.as_str().ok_or_else(|| {
                ExecutionError::Configuration(
                    "Every argument must be a string; numbers and JSON values must be quoted."
                        .into(),
                )
            })?;
            if value.len() > MAX_ARGUMENT_BYTES {
                return Err(ExecutionError::Configuration(format!(
                    "An argument is longer than {MAX_ARGUMENT_BYTES} bytes."
                )));
            }
            Ok(value.to_string())
        })
        .collect()
}

fn summary(stdout: &[u8], stderr: &[u8]) -> String {
    let output = if !stderr.is_empty() { stderr } else { stdout };
    let text = String::from_utf8_lossy(output).replace(['\n', '\r'], " ");
    let text = text.trim();
    let mut end = text.len().min(120);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let suffix = if text.len() > end { "…" } else { "" };
    format!("{}{}", &text[..end], suffix)
}

async fn execute(
    settings: &CommandSettings,
    arguments: Vec<String>,
) -> Result<String, ExecutionError> {
    let mut command = Command::new(settings.executable());
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if !settings.working_directory.trim().is_empty() {
        command.current_dir(PathBuf::from(settings.working_directory.trim()));
    }
    let seconds = settings.timeout().as_secs();
    let output = tokio::time::timeout(settings.timeout(), command.output())
        .await
        .map_err(|_| ExecutionError::TimedOut(seconds))?
        .map_err(|error| {
            ExecutionError::Process(format!(
                "Could not start {}: {error}",
                settings.executable()
            ))
        })?;
    let output_summary = summary(&output.stdout, &output.stderr);
    if output.status.success() {
        Ok(output_summary)
    } else {
        Err(ExecutionError::Failed {
            code: output.status.code(),
            output: output_summary,
        })
    }
}

async fn report(
    instance: &Instance,
    settings: &CommandSettings,
    result: Result<String, ExecutionError>,
    to_inspector: bool,
) {
    match result {
        Ok(output) => {
            let title = if settings.show_output && !output.is_empty() {
                output.clone()
            } else {
                settings.title().unwrap_or_else(|| "Done".to_string())
            };
            let _ = instance.set_title(Some(title), None).await;
            let _ = instance.show_ok().await;
            if to_inspector {
                let _ = instance
                    .send_to_property_inspector(
                        json!({"kind":"result", "ok":true, "message": output}),
                    )
                    .await;
            }
        }
        Err(error) => {
            let message = error.to_string();
            log::warn!("LightCraft CLI action failed: {message}");
            let _ = instance.set_title(Some("Error".to_string()), None).await;
            let _ = instance.show_alert().await;
            if to_inspector {
                let _ = instance
                    .send_to_property_inspector(
                        json!({"kind":"result", "ok":false, "message": message}),
                    )
                    .await;
            }
        }
    }
}

async fn run_action(instance: &Instance, settings: &CommandSettings, to_inspector: bool) {
    let result = parse_arguments(&settings.arguments).and_then(|arguments| {
        // Validation above is synchronous; the process work stays on the async runtime.
        Ok(arguments)
    });
    let result = match result {
        Ok(arguments) => execute(settings, arguments).await,
        Err(error) => Err(error),
    };
    report(instance, settings, result, to_inspector).await;
}

fn catalog_items(stdout: &str) -> Vec<Value> {
    let Ok(value) = serde_json::from_str::<Value>(stdout) else {
        return Vec::new();
    };
    let items = value
        .as_array()
        .cloned()
        .or_else(|| value.get("commands").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    items
        .into_iter()
        .filter_map(|item| {
            if let Some(id) = item.as_str() {
                return Some(json!({"id": id, "label": id}));
            }
            let id = ["id", "command", "tool", "name"]
                .into_iter()
                .find_map(|key| item.get(key).and_then(Value::as_str))?;
            let label = item
                .get("label")
                .or_else(|| item.get("name"))
                .and_then(Value::as_str)
                .unwrap_or(id);
            Some(json!({"id": id, "label": label}))
        })
        .collect()
}

async fn send_catalog(instance: &Instance, settings: &CommandSettings) {
    let mut command = Command::new(settings.executable());
    command
        .args(CATALOG_ARGUMENTS)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if !settings.working_directory.trim().is_empty() {
        command.current_dir(PathBuf::from(settings.working_directory.trim()));
    }
    let result = tokio::time::timeout(Duration::from_secs(15), command.output()).await;
    let reply = match result {
        Ok(Ok(output)) if output.status.success() && output.stdout.len() <= MAX_CATALOG_BYTES => {
            let text = String::from_utf8_lossy(&output.stdout);
            let items = catalog_items(&text);
            if items.is_empty() {
                json!({"kind":"catalog", "ok":false, "message":"The installed CLI did not return a JSON command catalog."})
            } else {
                json!({"kind":"catalog", "ok":true, "items":items})
            }
        }
        Ok(Ok(output)) => {
            json!({"kind":"catalog", "ok":false, "message":format!("Could not read the command catalog: {}", summary(&output.stdout, &output.stderr))})
        }
        Ok(Err(error)) => {
            json!({"kind":"catalog", "ok":false, "message":format!("Could not start {}: {error}", settings.executable())})
        }
        Err(_) => {
            json!({"kind":"catalog", "ok":false, "message":"Timed out while loading the command catalog."})
        }
    };
    let _ = instance.send_to_property_inspector(reply).await;
}

struct RunLightCraftCli;

#[async_trait]
impl Action for RunLightCraftCli {
    const UUID: &'static str = "wt.luizgustavo.getcraft.lightcraft.run";
    type Settings = CommandSettings;

    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        run_action(instance, settings, false).await;
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.key_down(instance, settings).await
    }

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if let Some(title) = settings.title() {
            let _ = instance.set_title(Some(title), None).await;
        }
        Ok(())
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        let effective = payload
            .get("settings")
            .and_then(|value| serde_json::from_value::<CommandSettings>(value.clone()).ok())
            .unwrap_or_else(|| settings.clone());
        match payload.get("request").and_then(Value::as_str) {
            Some("catalog") => send_catalog(instance, &effective).await,
            Some("test") => run_action(instance, &effective, true).await,
            _ => {}
        }
        Ok(())
    }
}

/// These controls use `lightcraft-cli run --connect`, so they act on the selected photos and the
/// UI the photographer is looking at. A CLI action without this bridge would open another library
/// and be much less useful from a Stream Deck.
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct LightControlSettings {
    executable: String,
    control_address: String,
    working_directory: String,
    control: String,
    rating: u8,
    color_label: String,
    preset_id: String,
    title: String,
    timeout_seconds: u64,
}

impl Default for LightControlSettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            control_address: "127.0.0.1:7980".into(),
            working_directory: String::new(),
            control: "pick".into(),
            rating: 5,
            color_label: "green".into(),
            preset_id: String::new(),
            title: String::new(),
            timeout_seconds: 15,
        }
    }
}

impl LightControlSettings {
    fn command_settings(&self) -> CommandSettings {
        CommandSettings {
            executable: self.executable.clone(),
            arguments: DEFAULT_ARGUMENTS.into(),
            working_directory: self.working_directory.clone(),
            title: self.title.clone(),
            show_output: false,
            timeout_seconds: self.timeout_seconds,
        }
    }

    fn address(&self) -> Result<&str, ExecutionError> {
        let address = self.control_address.trim();
        if address.is_empty() || !address.contains(':') {
            return Err(ExecutionError::Configuration(
                "Control address must be host:port, for example 127.0.0.1:7980.".into(),
            ));
        }
        Ok(address)
    }

    fn title_or(&self, fallback: &str) -> String {
        if self.title.trim().is_empty() {
            fallback.into()
        } else {
            self.title.trim().into()
        }
    }
}

fn light_control_command(
    settings: &LightControlSettings,
) -> Result<(String, Value, String), ExecutionError> {
    let label = if settings.color_label.trim().is_empty() {
        "green"
    } else {
        settings.color_label.trim()
    };
    let (command, params, title) = match settings.control.as_str() {
        "rate" => (
            "photo.rate",
            json!({"rating": settings.rating.min(5)}),
            "Rate Photo",
        ),
        "label" => ("photo.label", json!({"label": label}), "Color Label"),
        "pick" => ("photo.pick", json!({}), "Pick"),
        "reject" => ("photo.reject", json!({}), "Reject"),
        "next" => ("library.next", json!({}), "Next Photo"),
        "previous" => ("library.previous", json!({}), "Previous Photo"),
        "beforeAfter" => (
            "ui.menu.invoke",
            json!({"id":"view.beforeAfter"}),
            "Before / After",
        ),
        "beforeAfterSplit" => (
            "ui.menu.invoke",
            json!({"id":"view.beforeAfterSplit"}),
            "Before / After",
        ),
        "crop16x9" => ("crop.aspect", json!({"aspect":"16x9"}), "Crop 16:9"),
        "crop1x1" => ("crop.aspect", json!({"aspect":"1x1"}), "Crop 1:1"),
        "crop4x5" => ("crop.aspect", json!({"aspect":"4x5"}), "Crop 4:5"),
        "autoStraighten" => ("crop.autoStraighten", json!({}), "Auto Straighten"),
        "resetCrop" => ("crop.reset", json!({}), "Reset Crop"),
        "applyPreset" if !settings.preset_id.trim().is_empty() => (
            "preset.apply",
            json!({"id":settings.preset_id.trim()}),
            "Apply Preset",
        ),
        "applyPreset" => {
            return Err(ExecutionError::Configuration(
                "Enter a preset ID before using Apply Preset.".into(),
            ));
        }
        "openMasking" => ("ui.menu.invoke", json!({"id":"panel.masking"}), "Masking"),
        "addSubjectMask" => ("mask.add", json!({"kind":"subject"}), "Subject Mask"),
        "exportDialog" => ("ui.menu.invoke", json!({"id":"dialog.export"}), "Export"),
        "exportPrevious" => (
            "ui.menu.invoke",
            json!({"id":"app.exportPrevious"}),
            "Export Previous",
        ),
        "undo" => ("edit.undo", json!({}), "Undo"),
        "redo" => ("edit.redo", json!({}), "Redo"),
        other => {
            return Err(ExecutionError::Configuration(format!(
                "Unknown LightCraft control `{other}`. Choose a control in the Property Inspector."
            )));
        }
    };
    Ok((command.into(), params, settings.title_or(title)))
}

fn light_run_arguments(
    settings: &LightControlSettings,
    command: &str,
    params: Value,
) -> Result<Vec<String>, ExecutionError> {
    Ok(vec![
        "run".into(),
        "--connect".into(),
        settings.address()?.into(),
        command.into(),
        serde_json::to_string(&params)
            .map_err(|error| ExecutionError::Configuration(error.to_string()))?,
    ])
}

async fn report_light_live(
    instance: &Instance,
    title: String,
    result: Result<String, ExecutionError>,
) {
    match result {
        Ok(_) => {
            let _ = instance.set_title(Some(title), None).await;
            let _ = instance.show_ok().await;
        }
        Err(error) => {
            log::warn!("LightCraft live control failed: {error}");
            let _ = instance.set_title(Some("Error"), None).await;
            let _ = instance.show_alert().await;
        }
    }
}

struct LightCraftControl;

#[async_trait]
impl Action for LightCraftControl {
    const UUID: &'static str = "wt.luizgustavo.getcraft.lightcraft.control";
    type Settings = LightControlSettings;

    async fn key_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let result = match light_control_command(settings) {
            Ok((command, params, title)) => match light_run_arguments(settings, &command, params) {
                Ok(args) => {
                    report_light_live(
                        instance,
                        title,
                        execute(&settings.command_settings(), args).await,
                    )
                    .await;
                    return Ok(());
                }
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        report_light_live(instance, "Error".into(), result).await;
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.key_down(instance, settings).await
    }

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if let Ok((_, _, title)) = light_control_command(settings) {
            let _ = instance.set_title(Some(title), None).await;
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct LightDialSettings {
    executable: String,
    control_address: String,
    working_directory: String,
    control: String,
    custom_control: String,
    step: f64,
    press_action: String,
    title: String,
    timeout_seconds: u64,
}

impl Default for LightDialSettings {
    fn default() -> Self {
        Self {
            executable: String::new(),
            control_address: "127.0.0.1:7980".into(),
            working_directory: String::new(),
            control: "light.exposure".into(),
            custom_control: String::new(),
            step: 0.1,
            press_action: "beforeAfter".into(),
            title: String::new(),
            timeout_seconds: 15,
        }
    }
}

impl LightDialSettings {
    fn as_control_settings(&self) -> LightControlSettings {
        LightControlSettings {
            executable: self.executable.clone(),
            control_address: self.control_address.clone(),
            working_directory: self.working_directory.clone(),
            title: self.title.clone(),
            timeout_seconds: self.timeout_seconds,
            ..Default::default()
        }
    }

    fn develop_control(&self) -> Result<(&str, &str), ExecutionError> {
        match self.control.as_str() {
            "light.exposure" => Ok(("light.exposure", "Exposure")),
            "wb.temp" => Ok(("wb.temp", "Temperature")),
            "wb.tint" => Ok(("wb.tint", "Tint")),
            "light.contrast" => Ok(("light.contrast", "Contrast")),
            "light.highlights" => Ok(("light.highlights", "Highlights")),
            "light.shadows" => Ok(("light.shadows", "Shadows")),
            "effects.clarity" => Ok(("effects.clarity", "Clarity")),
            "effects.dehaze" => Ok(("effects.dehaze", "Dehaze")),
            "color.vibrance" => Ok(("color.vibrance", "Vibrance")),
            "crop.angle" => Ok(("crop.angle", "Straighten")),
            "custom" if !self.custom_control.trim().is_empty() => {
                Ok((self.custom_control.trim(), "Develop"))
            }
            "custom" => Err(ExecutionError::Configuration(
                "Enter a develop control ID for a Custom dial.".into(),
            )),
            other => Err(ExecutionError::Configuration(format!(
                "Unknown develop control `{other}`."
            ))),
        }
    }

    fn step(&self) -> Result<f64, ExecutionError> {
        if !self.step.is_finite() || self.step == 0.0 {
            return Err(ExecutionError::Configuration(
                "Dial step must be a non-zero number.".into(),
            ));
        }
        Ok(self.step.clamp(-1_000_000.0, 1_000_000.0))
    }
}

fn light_dial_press(
    settings: &LightDialSettings,
) -> Result<Option<(String, Value, String)>, ExecutionError> {
    let control = settings.as_control_settings();
    let action: (String, Value, String) = match settings.press_action.as_str() {
        "none" => return Ok(None),
        "beforeAfter" => (
            "ui.menu.invoke".into(),
            json!({"id":"view.beforeAfter"}),
            "Before / After".into(),
        ),
        "masking" => (
            "ui.menu.invoke".into(),
            json!({"id":"panel.masking"}),
            "Masking".into(),
        ),
        "export" => (
            "ui.menu.invoke".into(),
            json!({"id":"dialog.export"}),
            "Export".into(),
        ),
        "undo" => ("edit.undo".into(), json!({}), "Undo".into()),
        other => {
            return Err(ExecutionError::Configuration(format!(
                "Unknown dial press action `{other}`."
            )))
        }
    };
    Ok(Some((action.0, action.1, control.title_or(&action.2))))
}

struct LightCraftDial;

#[async_trait]
impl Action for LightCraftDial {
    const UUID: &'static str = "wt.luizgustavo.getcraft.lightcraft.dial";
    type Settings = LightDialSettings;

    async fn dial_rotate(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        ticks: i16,
        _pressed: bool,
    ) -> OpenActionResult<()> {
        if ticks == 0 {
            return Ok(());
        }
        let result = (|| -> Result<(Vec<String>, String), ExecutionError> {
            let (control, label) = settings.develop_control()?;
            let delta = settings.step()? * f64::from(ticks);
            let common = settings.as_control_settings();
            let args = light_run_arguments(
                &common,
                "develop.adjust",
                json!({"control":control, "delta":delta}),
            )?;
            let title: String = if settings.title.trim().is_empty() {
                format!("{label}\n{delta:+.2}")
            } else {
                settings.title.trim().into()
            };
            Ok((args, title))
        })();
        let common = settings.as_control_settings();
        match result {
            Ok((args, title)) => {
                report_light_live(
                    instance,
                    title,
                    execute(&common.command_settings(), args).await,
                )
                .await
            }
            Err(error) => report_light_live(instance, "Error".into(), Err(error)).await,
        }
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let common = settings.as_control_settings();
        match light_dial_press(settings) {
            Ok(Some((command, params, title))) => {
                match light_run_arguments(&common, &command, params) {
                    Ok(args) => {
                        report_light_live(
                            instance,
                            title,
                            execute(&common.command_settings(), args).await,
                        )
                        .await
                    }
                    Err(error) => report_light_live(instance, "Error".into(), Err(error)).await,
                }
            }
            Ok(None) => {}
            Err(error) => report_light_live(instance, "Error".into(), Err(error)).await,
        }
        Ok(())
    }

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if let Ok((_, label)) = settings.develop_control() {
            let title: String = if settings.title.trim().is_empty() {
                label.into()
            } else {
                settings.title.trim().into()
            };
            let _ = instance.set_title(Some(title), None).await;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let _ = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Never,
    );
    register_action(RunLightCraftCli).await;
    register_action(LightCraftControl).await;
    register_action(LightCraftDial).await;
    run(std::env::args().collect()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_string_argument_vector() {
        assert_eq!(
            parse_arguments(r#"["exec","timeline.play","{}"]"#).unwrap(),
            ["exec", "timeline.play", "{}"]
        );
    }

    #[test]
    fn rejects_non_string_arguments() {
        assert!(parse_arguments(r#"["exec", 4]"#).is_err());
    }

    #[test]
    fn stderr_is_preferred_in_a_failure_summary() {
        assert_eq!(summary(b"normal", b"bad\nnews"), "bad news");
    }

    #[test]
    fn has_a_usable_default_command() {
        assert_eq!(
            parse_arguments(&CommandSettings::default().arguments).unwrap(),
            CATALOG_ARGUMENTS
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn honors_a_one_second_timeout() {
        assert_eq!(
            CommandSettings {
                timeout_seconds: 1,
                ..Default::default()
            }
            .timeout(),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn live_control_routes_before_after_to_the_desktop_ui() {
        let settings = LightControlSettings {
            control: "beforeAfter".into(),
            ..Default::default()
        };
        let (command, params, _) = light_control_command(&settings).unwrap();
        assert_eq!(command, "ui.menu.invoke");
        assert_eq!(params, json!({"id":"view.beforeAfter"}));
        let arguments = light_run_arguments(&settings, &command, params).unwrap();
        assert_eq!(arguments[..3], ["run", "--connect", "127.0.0.1:7980"]);
    }

    #[test]
    fn develop_dial_keeps_a_relative_adjustment() {
        let settings = LightDialSettings {
            control: "light.exposure".into(),
            step: 0.1,
            ..Default::default()
        };
        assert_eq!(settings.develop_control().unwrap().0, "light.exposure");
        assert!((settings.step().unwrap() * 3.0 - 0.3).abs() < f64::EPSILON);
        assert!(LightDialSettings {
            control: "custom".into(),
            ..Default::default()
        }
        .develop_control()
        .is_err());
    }
}
