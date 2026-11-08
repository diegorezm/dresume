# Documentação

O dresume guarda o **conteúdo** do seu currículo em YAML e o **layout** em templates Typst. Mude um template
e todos os currículos ligados a ele mudam junto. Esta página cobre o app desktop, o formato do workspace,
os templates e a CLI.

## Começando {#get-started}

> Instaladores prontos chegam em breve. Por enquanto, compile a partir do código.

Você precisa de [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/) e [pnpm](https://pnpm.io/).
**Não** é preciso instalar o Typst: ele já vem embutido.

```bash
git clone https://github.com/diegorezm/dresume
cd dresume/app
pnpm install
pnpm tauri dev
```

Clique em **Novo workspace**, escolha uma pasta e comece a editar `my-resume`.

### Só linha de comando

```bash
cargo install --path crates/cli
dresume init my-resumes
dresume -w my-resumes build          # → my-resumes/output/*.pdf
```

## Conceitos {#concepts}

Um **workspace** é uma pasta comum. Não há banco de dados nem estado escondido: tudo é arquivo que você pode
ler, editar, comparar e versionar.

```
my-resumes/
├── dresume.yaml        marcador do workspace (nome, versão)
├── profile.yaml        nome, email, links — compartilhado por todos os currículos
├── templates/<id>/
│   ├── template.yaml   nome, descrição, campos do perfil ocultos
│   ├── template.typ    o layout em Typst
│   ├── schema.json     qual conteúdo o template aceita
│   └── sample.yaml     conteúdo de exemplo para pré-visualizar
├── resumes/<id>.yaml   só conteúdo, com `template: <id>`
└── fonts/              fontes extras opcionais (.ttf/.otf)
```

- Um **template** é layout mais um JSON Schema descrevendo o conteúdo que aceita.
- Um **currículo** é só conteúdo e se liga a um template com `template: <id>`.
- O **perfil** guarda seus dados pessoais uma vez. Cada template pode ocultar os campos que não quer.
- Ids (nomes de arquivos e pastas) usam só letras, números, `-` e `_`.

## Usando o app {#app}

### Visão geral

A tela inicial mostra seus templates como cards, cada um com os currículos que o usam. A partir de um card
você cria um currículo com aquele template ou abre o layout do template.

### Editando um currículo

Currículos são editados num formulário gerado a partir do schema do template: campos básicos no topo e um
card por seção. Arraste itens para reordenar, adicione tópicos e tags, e inclua seções opcionais quando
precisar. A pré-visualização atualiza enquanto você digita, antes de salvar. Exporte para PDF pela barra superior.

### Editando um template

O editor de templates tem uma aba por arquivo (`template.typ`, auxiliares, `schema.json`, `sample.yaml`) com
destaque de sintaxe Typst. A pré-visualização usa o próprio `sample.yaml` do template, e os erros apontam
arquivo e linha.

### Perfil

O editor de perfil edita o `profile.yaml`. Links podem ser URLs simples ou `{label, url}`. A tabela
**mostrar nos templates** escolhe quais campos cada template exibe.

### Painel do agente de IA: Claude e ChatGPT {#agent-panel}

Aperte <kbd>Ctrl</kbd>+<kbd>J</kbd> no editor de currículo ou de template. O painel usa um agente de IA que
você já instalou e logou: **Claude Code**, **Codex** (com sua conta do ChatGPT), **Gemini CLI** ou
**OpenCode**. O dresume nunca chama uma API de modelo por conta própria e não precisa de chave de API.

1. Instale um agente e faça login uma vez (por exemplo `claude` ou `codex login`).
2. Abra um currículo e aperte <kbd>Ctrl</kbd>+<kbd>J</kbd>.
3. Peça uma mudança: *"adapte este currículo para esta vaga: …"*, *"traduza para inglês"*,
   *"deixe em uma página"*. No editor de template: *"faça em duas colunas com barra lateral escura"*.
4. O agente edita os arquivos e a pré-visualização acompanha. Revise o diff e **Mantenha** ou **Desfaça** cada mudança.

Escolha o modelo e o nível de raciocínio no campo de mensagem. As **Permissões** (também nos Ajustes)
definem o que o agente pode fazer:

- **Perguntar antes**: toda edição e comando esperam sua aprovação.
- **Editar sozinho**: edições nos arquivos do workspace passam; comandos perguntam antes.
- **Somente leitura**: o agente pode ler e sugerir, mas toda mudança é recusada.

### Claude Desktop e outros clientes MCP {#mcp}

O `dresume mcp` serve um workspace via MCP (stdio), então qualquer cliente MCP consegue listar, validar,
mostrar, gerar e pré-visualizar currículos e templates. O app já conecta isso às sessões de agente quando a
CLI está no PATH.

```bash
claude mcp add dresume -- dresume -w ~/my-resumes mcp
```

No Claude Desktop, adicione ao `claude_desktop_config.json`:

```json
{ "mcpServers": { "dresume": { "command": "dresume", "args": ["-w", "/caminho/para/my-resumes", "mcp"] } } }
```

## Ajustes {#settings}

Os ajustes são salvos por usuário, na pasta de configuração do app, não no workspace.

{{settings}}

### Temas

{{themes}}

### Temas personalizados

Use **Salvar como novo tema** para guardar o tema atual com sua cor de destaque, ou coloque um arquivo JSON
na pasta de temas (**Abrir pasta de temas** nos Ajustes). O formato é o mesmo dos temas embutidos:

{{themeJson}}

## Atalhos de teclado {#shortcuts}

No macOS use <kbd>Cmd</kbd> em vez de <kbd>Ctrl</kbd>.

{{shortcuts}}

## Criando templates {#templates}

Templates recebem os dados por duas entradas com caminhos relativos ao workspace:

```typst
#let profile = yaml(sys.inputs.profile)
#let resume  = yaml(sys.inputs.resume)
```

O perfil que o template recebe é normalizado: links sempre chegam como `{label, url}` e os campos ocultos
pelo template são removidos. Renderize os campos de contato de forma genérica (percorrendo o perfil) para
que campos novos ou ocultos nunca quebrem o layout. Ao adicionar um campo, inclua-o também no `schema.json`
e no `sample.yaml`.

Comece pelo **Basic** (`--from @starter`) ou copie um template existente. Para fontes que funcionam em qualquer
lugar, use as embutidas no Typst: Libertinus Serif, New Computer Modern, DejaVu Sans Mono. Outras vão em `fonts/`.

### Schema e o formulário

Estas palavras-chave do schema controlam o formulário do app:

| Palavra-chave | Efeito |
|---|---|
| `title` | Rótulo do campo |
| `format: "textarea"` | Texto em várias linhas |
| `enum` | Lista de seleção |
| `format: "uri"` | Campo de URL |
| array de strings | Tópicos |
| `format: "tags"` num array de strings | Chips em vez de tópicos |
| array de objetos | Cards recolhíveis, com título dos dois primeiros campos de texto |
| chave `period` / `date` / `year` | Aparece como detalhe do card |
| `items.title` | Nome no singular ("Adicionar cargo") |

Campos de topo que não são arrays ficam no card **Básico**; seções opcionais aparecem em **Adicionar seção**.

## Referência da CLI {#cli}

Todo comando aceita `-w <dir>` (workspace; por padrão o diretório pai mais próximo com `dresume.yaml`) e
`--json` para saída legível por máquina. Os comandos retornam código diferente de zero em caso de falha.
As descrições abaixo vêm direto do código, em inglês.

{{cli}}

## Para agentes de IA {#agents}

Todo workspace inclui um `AGENTS.md` que ensina o formato aos agentes. Um ciclo típico:

1. Editar o YAML.
2. `dresume validate --json` para conferir contra o schema.
3. `dresume build <id> --format svg --json` para gerar páginas que o agente consegue ver.
4. Corrigir os diagnósticos reportados.

Experimente pedidos como *"adapte meu currículo de backend para esta vaga"* ou *"crie um template em duas colunas com barra lateral escura"*.
