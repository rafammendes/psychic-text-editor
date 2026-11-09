# Psychic Text Editor

Um editor de texto rápido, leve, gratuito e de código aberto para Linux, inspirado no **Sublime Text** e escrito 100% em **Rust** (`eframe` / `egui` + `syntect`).

Sem Electron, sem telemetria, sem contas/logins obrigatórios e com abertura instantânea via renderização acelerada por GPU (OpenGL/Glow).

## Funcionalidades (MVP)

- **Abertura Instantânea e Leveza**: Binário nativo em Rust com consumo mínimo de RAM e CPU.
- **Fonte Embutida (Zero Configuração)**: Vem com a **JetBrains Mono Nerd Font** (licença aberta SIL OFL 1.1) embutida diretamente no binário, garantindo ícones de arquivos/pastas e excelente legibilidade de código em qualquer distribuição Linux.
- **Syntax Highlighting Estilo Sublime**: Motor `syntect` baseado em gramáticas TextMate (o mesmo padrão usado pelo Sublime Text), com cache de layout e suporte a arquivos de código e de configuração Linux (`.conf`, `.ini`, `.toml`, `.service`, `.sh`, `.rs`, `.py`, `.json`, `.yaml`, `.md`, etc.).
- **Command Palette (`Ctrl+Shift+P`)**: Acesso rápido por busca fuzzy a todos os comandos, troca de sintaxe e seleção de temas.
- **Goto Anything / Quick Open (`Ctrl+P`)**: Abra qualquer arquivo do diretório rapidamente com busca fuzzy ou pule para uma linha usando `:numero` (ex: `main.rs:40` ou `:120`).
- **Sidebar de Arquivos (`Ctrl+B`)**: Árvore de diretórios lateral com ícones Nerd Font e botão para exibir/ocultar dotfiles (`.config`, `.bashrc`, etc.).
- **Múltiplas Abas e Indicador de Alteração (`•`)**: Gerenciamento fluido de abas com suporte a clique do meio para fechar e alternância rápida de quebra de linha (`LF` / `CRLF`).
- **Busca e Substituição (`Ctrl+F` / `Ctrl+H`)**: Barra inferior rápida para localizar e substituir texto com opção case-sensitive (`Aa`).

## Atalhos de Teclado

| Atalho | Ação |
| :--- | :--- |
| `Ctrl+Shift+P` | Abrir **Command Palette** |
| `Ctrl+P` | **Goto Anything / Quick Open** (arquivos ou `:linha`) |
| `Ctrl+N` | Novo arquivo (Nova aba) |
| `Ctrl+O` | Abrir arquivo por caminho |
| `Ctrl+K` | Abrir pasta / projeto na Sidebar |
| `Ctrl+S` | Salvar arquivo atual |
| `Ctrl+Shift+S` | Salvar como... |
| `Ctrl+W` | Fechar aba atual |
| `Ctrl+B` | Mostrar / Ocultar Sidebar |
| `Ctrl+F` / `Ctrl+H` | Buscar e Substituir |
| `Ctrl+G` | Ir para linha (Goto Line) |
| `Alt+Z` | Alternar quebra de linha automática (Word Wrap) |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Aumentar / Diminuir / Resetar tamanho da fonte |

## Como Compilar e Executar

```bash
# Executar em modo de desenvolvimento
cargo run --release

# Abrir um arquivo ou pasta diretamente pelo terminal
cargo run --release -- /etc/fstab
cargo run --release -- .
```

## Licenças

- **Código-fonte**: MIT License
- **Fonte embutida**: [JetBrains Mono](https://www.jetbrains.com/lp/mono/) + [Nerd Fonts](https://www.nerdfonts.com/) sob **SIL Open Font License 1.1 (OFL-1.1)** — livre para uso, redistribuição e empacotamento comercial ou aberto.
