# LightCraft Live Controls for OpenDeck

[Português do Brasil](#português-do-brasil) · [Contributing](CONTRIBUTING.md) · [LightCraft](https://github.com/storytold/lightcraft) · [OpenDeck](https://github.com/nekename/OpenDeck)

An independent Linux-only [OpenDeck](https://github.com/nekename/OpenDeck) plugin that turns a Stream Deck, Stream Deck+, or D200X-style encoder deck into a practical live control surface for [LightCraft](https://github.com/storytold/lightcraft). Cull with picks, rejects, ratings, and labels; develop with Before/After, crop, masks, presets, export controls, and relative adjustment dials for exposure, temperature, contrast, and more.

The dedicated controls connect to the running LightCraft application, so they operate on the selected photo(s) in the library already on screen—even when that library is open and locked by LightCraft. The configurable `lightcraft-cli` action remains an escape hatch for uncommon commands, rather than the primary deck experience.

Each action stores an executable path and a JSON array of arguments. The plugin launches the executable directly, without `sh -c` or another shell. That makes JSON parameters, spaces in paths, and untrusted filenames safe to pass as arguments.

## Features

- **Open LightCraft** starts the desktop app with the selected loopback host and control port, ready for live library and Develop Dial controls.
- **LightCraft Live Control**: pick/reject, rating, color label, next/previous photo, Before/After, crop ratios, auto straighten, presets, masking, and export actions.
- **LightCraft Develop Dial** for Stream Deck+ and Ulanzi D200X-style encoders: relative exposure, temperature, tint, tone, color, dehaze, clarity and straighten adjustments. Encoder press can toggle Before/After, open Masking/Export, or undo.
- The live controls act on the selected photo(s) in the running LightCraft application, including a library already locked by that app.
- One configurable **Run LightCraft CLI** escape hatch for keypad keys and encoders.
- Supports every LightCraft CLI subcommand, option, and future addition: the action forwards the exact JSON argument array you configure.
- Loads the installed CLI's command or tool catalog in the Property Inspector and creates an app-specific, editable action template.
- Configurable executable path, working directory, timeout (1–300 seconds), label, and optional output summary on the key.
- Test a configuration in the Property Inspector before assigning it to a key.
- Linux x86_64 release ZIP created on every CI run; tagged versions are published as GitHub Releases.

## Real photo workflows

- **Cull an event quickly:** keep Pick, Reject, 5 Stars, Green Label, and Next Photo together so selection decisions happen without moving from the keyboard or controller.
- **Develop without overwriting work:** turn the Exposure, Temperature, or Contrast dial for small relative changes; press the encoder for a Before/After check, then open Masking when local work is needed.
- **Finish a delivery:** set a crop ratio or auto-straighten, apply a known preset, then open Export from the deck. “Export with previous settings” is deliberately separate because it writes files.

## Requirements

- Linux x86_64
- OpenDeck with OpenAction-compatible plugin support
- LightCraft and `lightcraft-cli` installed and available in `PATH`, or its absolute path entered in the Property Inspector
- For live controls, launch the app with `lightcraft --control 7980` (or set the matching address in each action)

## Install

1. Download `getcraft-lightcraft-opendeck-linux-x86_64.zip` from the [latest release](../../releases/latest).
2. In OpenDeck, open **Plugins** → **Install from file** and select the ZIP. Do not extract or rename the `.sdPlugin` directory.
3. Drag **Open LightCraft** to a key, set the desktop executable and control host/port, then press it.
4. Drag **LightCraft Live Control** to culling, crop, masking and export keys. Add **LightCraft Develop Dial** to an encoder using the same address.
5. The generic **Run LightCraft CLI** action remains available for every other CLI command.

## Configuration and examples

The **Arguments** field must contain a JSON array of strings. It is not a shell command.

| Goal | Arguments |
| --- | --- |
| List the installed command catalog | `["commands", "--json"]` |

Use the exact argument syntax shown by `lightcraft-cli --help` for project, file, export, and live-app control operations. The generic array accepts every documented subcommand and option.

The Property Inspector's **Load command catalog** button asks the installed CLI for the catalog supported by this application. Selecting an ID fills an app-specific, editable argument template; use **Test command** after adding the parameters and options required by your installed CLI version. The generic JSON array remains the canonical path to every CLI capability.

## Suggested D200X / Stream Deck+ layout

| Control | Suggested action |
| --- | --- |
| Key 1 / 2 | Pick / Reject |
| Key 3 | Rating: 5 stars |
| Key 4 | Color label: Green |
| Key 5 | Before / After |
| Key 6 | Crop 16:9 |
| Key 7 | Auto straighten |
| Key 8 | Open Masking panel |
| Key 9 | Open Export dialog |
| Encoder 1 | Develop Dial: Exposure, step 0.10 |
| Encoder 2 | Develop Dial: Temperature, step 100 |
| Encoder 3 | Develop Dial: Contrast, step 1 |

Each dial uses LightCraft's `develop.adjust` command, so it adds a relative delta and keeps all other edits intact. Avoid **Export with previous settings** until a previous export is configured; it deliberately writes files.

## CI/CD and release

Pull requests and pushes to `main` run formatting checks, unit tests, and `scripts/package.sh`. The workflow uploads the Linux installer ZIP as an artifact. Pushing a semantic tag such as `v0.1.0` publishes the same ZIP as a GitHub Release.

To package locally:

```bash
rustup target add x86_64-unknown-linux-gnu
bash scripts/package.sh
```

The output is `dist/getcraft-lightcraft-opendeck-linux-x86_64.zip`, containing exactly `wt.luizgustavo.getcraft.lightcraft.sdPlugin/` at its root.

## Local test installation

On Linux, build the local installation package with:

```bash
bash scripts/install-local.sh
```

By default, the installer only builds the ZIP and does not change OpenDeck's active plugin directory. This prevents development plugins from disrupting an active setup. To deliberately test one plugin in the active OpenDeck installation, opt in explicitly:

```bash
OPENDECK_ALLOW_LIVE_INSTALL=1 bash scripts/install-local.sh
```

Install one development plugin at a time, then use OpenDeck's plugin reload control or restart it manually. The installer itself never restarts or reloads OpenDeck. To target a separate configuration, set `OPENDECK_PLUGINS_DIR` before running the command.

## Platform roadmap

| Platform | Status |
| --- | --- |
| Linux x86_64 | Supported now |
| Windows | Planned; not implemented or packaged yet |
| macOS | Planned; not implemented or packaged yet |

## Security and governance

The plugin never evaluates a shell command. It executes the chosen binary with the strings in the JSON array, and it limits a process to five minutes.

This is an open-contribution repository. Direct write/merge access is reserved for `LuizGustavoWT`; outside contributors use forks and pull requests. GitHub does not have a separate permission that only controls closing issues: users without repository write/triage access cannot close other users' issues, so retaining those permissions only for the maintainer satisfies the governance model. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. LightCraft and OpenDeck are independent upstream projects with their own licenses and release policies.

---

# Português do Brasil

Plugin independente, somente para Linux, do [OpenDeck](https://github.com/nekename/OpenDeck) que transforma Stream Deck, Stream Deck+ ou uma mesa com encoders no estilo D200X em uma superfície de controle ao vivo do [LightCraft](https://github.com/storytold/lightcraft). Faça culling com pick, reject, estrelas e etiquetas; revele com antes/depois, crop, máscaras, presets, exportação e dials de ajustes relativos de exposição, temperatura, contraste e mais.

As ações prontas se conectam ao LightCraft em execução e trabalham sobre as fotos selecionadas na biblioteca que já está aberta — inclusive quando essa biblioteca está bloqueada pelo aplicativo. A ação configurável `lightcraft-cli` continua como saída para comandos pouco comuns, e não como a experiência principal do deck.

Cada ação guarda o executável e uma lista JSON de argumentos. O plugin chama o executável diretamente, sem `sh -c` nem outro shell. Assim, JSON, caminhos com espaços e nomes de arquivos não confiáveis são passados com segurança.

## Recursos

- **Open LightCraft** abre o aplicativo com o host loopback e a porta de controle para os controles da biblioteca e do Develop Dial.
- **LightCraft Live Control**: pick/reject, estrelas, etiquetas de cor, navegação, antes/depois, crop, presets, máscaras e exportação.
- **LightCraft Develop Dial**: ajustes relativos de exposição, temperatura, matiz, tom, cor, dehaze, clarity e endireitamento; pressionar o encoder pode alternar antes/depois, abrir máscara/exportação ou desfazer.
- As ações ao vivo alteram a foto selecionada dentro do LightCraft aberto, inclusive quando a biblioteca está bloqueada pelo aplicativo.
- Ação configurável **Run LightCraft CLI** continua disponível para comandos avançados.
- Todos os subcomandos, opções e futuras adições da CLI: a ação repassa exatamente o vetor JSON configurado.
- Catálogo dinâmico de comandos ou ferramentas da CLI no Property Inspector, com modelo de ação editável e específico do aplicativo.
- Caminho do executável, diretório de trabalho, timeout (1–300 s), rótulo e resumo de saída configuráveis.
- Botão de teste no Property Inspector antes de associar a ação a uma tecla.
- ZIP Linux x86_64 gerado em toda execução da CI; tags são publicadas como Releases.

## Fluxos fotográficos reais

- **Fazer culling de um evento:** mantenha Pick, Reject, 5 estrelas, etiqueta verde e Próxima foto juntos para decidir seleções sem sair do teclado ou controlador.
- **Revelar sem sobrescrever o trabalho:** gire o dial de Exposição, Temperatura ou Contraste para ajustes relativos pequenos; pressione o encoder para conferir Antes/Depois e abra Máscaras quando precisar de edição local.
- **Finalizar uma entrega:** defina uma proporção de crop ou endireite automaticamente, aplique um preset conhecido e abra Exportar pelo deck. **Exportar com configurações anteriores** fica separado porque grava arquivos.

## Requisitos e instalação

Você precisa de Linux x86_64, OpenDeck compatível com OpenAction e LightCraft com `lightcraft-cli` no `PATH` (ou o caminho absoluto no Property Inspector). Para os controles ao vivo, abra o LightCraft com `lightcraft --control 7980`.

1. Baixe `getcraft-lightcraft-opendeck-linux-x86_64.zip` na [release mais recente](../../releases/latest).
2. No OpenDeck, abra **Plugins** → **Install from file** e escolha o ZIP. Não extraia nem renomeie a pasta `.sdPlugin`.
3. Arraste **Open LightCraft** para uma tecla, configure executável, host e porta, depois pressione-a.
4. Arraste **LightCraft Live Control** para as teclas e **LightCraft Develop Dial** para os encoders usando o mesmo endereço.
5. Use a ação genérica **Run LightCraft CLI** somente para um comando não coberto pelas ações prontas.

## Configuração

O campo **Arguments** precisa ser um vetor JSON de strings; ele não é um comando de shell.

| Objetivo | Arguments |
| --- | --- |
| Listar o catálogo de comandos instalado | `["commands", "--json"]` |

Use a sintaxe exata de `lightcraft-cli --help` para operações de projeto, arquivo, exportação e controle do aplicativo em execução. O vetor genérico aceita todos os subcomandos e opções documentados.

**Load command catalog** consulta a CLI instalada pelo catálogo compatível com este aplicativo. Ao escolher um ID, o inspetor preenche um modelo de argumentos específico e editável; use **Test command** depois de adicionar os parâmetros e opções exigidos pela versão instalada da CLI. O vetor JSON genérico continua sendo o caminho para toda a CLI.

## CI/CD, roadmap e contribuição

PRs e pushes em `main` executam formatação, testes e `scripts/package.sh`. O ZIP vira artefato; uma tag como `v0.1.0` cria uma GitHub Release.

Para gerar o pacote local no Linux, execute `bash scripts/install-local.sh`. Por padrão, o instalador gera o ZIP e não altera o diretório de plugins ativo do OpenDeck, protegendo a configuração em uso. Para testar deliberadamente um único plugin na instalação ativa, confirme explicitamente:

```bash
OPENDECK_ALLOW_LIVE_INSTALL=1 bash scripts/install-local.sh
```

Instale um plugin de desenvolvimento por vez e então recarregue-o pelo OpenDeck ou reinicie manualmente. O instalador jamais reinicia ou recarrega o OpenDeck. Para usar outra configuração, defina `OPENDECK_PLUGINS_DIR` antes do comando.

| Plataforma | Situação |
| --- | --- |
| Linux x86_64 | Suportado agora |
| Windows | Planejado; ainda não implementado nem empacotado |
| macOS | Planejado; ainda não implementado nem empacotado |

Contribuições são bem-vindas via fork e pull request. Apenas `LuizGustavoWT` possui acesso direto para merge e para fechar issues de terceiros. O GitHub não oferece uma permissão separada somente para fechar issues; sem acesso de escrita/triagem, colaboradores públicos não conseguem fechar issues de outras pessoas. Veja [CONTRIBUTING.md](CONTRIBUTING.md).

## Licença

MIT. LightCraft e OpenDeck são projetos upstream independentes, cada um com sua própria licença.
