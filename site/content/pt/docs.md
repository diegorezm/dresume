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

### Painel do agente de IA

Abra com <kbd>Ctrl</kbd>+<kbd>J</kbd> no editor de currículo. Ele usa o agente de IA que você já tem
instalado (Claude Code ou Codex): peça uma mudança e o agente edita os arquivos do workspace enquanto a
pré-visualização acompanha. O dresume nunca chama uma API de modelo por conta própria.

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
