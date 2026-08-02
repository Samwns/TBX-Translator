# Manual do Desenvolvedor - TPG Translator

Este documento detalha o funcionamento interno, arquitetura e uso da biblioteca GTK4 (via `gtk4-rs`) no projeto TPG Translator.

## Arquitetura Geral

O aplicativo é escrito em **Rust** e dividido em módulos, cada um com uma responsabilidade bem definida:

- `main.rs`: Ponto de entrada do aplicativo. Inicializa o ambiente GTK (via `gtk::Application`).
- `ui.rs`: Arquivo principal da Interface de Usuário. Constrói as janelas, widgets, e conecta os eventos principais.
- `app_config.rs`: Cuida do carregamento e salvamento das configurações do usuário em formato JSON, persistindo opções como temas, diretórios e modos de API.
- `renpy_extractor.rs`: Lida com a engenharia reversa de jogos da engine Ren'Py (injeção de scripts Python, extração de dumps, tradução via API em lotes, e regeração de `.rpy`).
- `unity_extractor.rs`: Lida com a extração e configuração para jogos da engine Unity, interagindo primariamente com o plugin AutoTranslator.
- `font_injector.rs`: Janela auxiliar e lógica de patch para injetar fontes customizadas (`.ttf`, `.otf`) nos motores, corrigindo problemas de renderização de caracteres acentuados.
- `api.rs`: Comunicação HTTP via `reqwest` com a Google API para tradução de blocos de texto.

---

## Como o App Funciona

### 1. Iniciação e Interface
Ao rodar o binário, o GTK inicializa a janela principal. A UI não usa construtores visuais (como Glade ou XML). Toda a interface é desenhada puramente via código Rust no `ui.rs`. A janela é "frameless" (decorada customizada), usando `GestureClick` para movimentação da barra de título e removendo os frames nativos do OS.

### 2. Seleção de Motor (Engine)
O usuário pode escolher abas diferentes para Ren'Py e Unity. Essa escolha define o `engine_mode`.
Dependendo da aba, o botão de "Traduzir" passa os comandos para `renpy_extractor` ou `unity_extractor`.

### 3. Extração e Tradução (Ren'Py)
1. **Injeção:** O App insere um script `desired_python.py` na estrutura interna do jogo e modifica o carregamento (por ex, `tpg_boot.rpy`) para forçar o jogo a ler e "dumpar" todos os textos de diálogo e interface na primeira vez que ele é aberto.
2. **Execução Headless:** O `renpy_extractor` inicia o executável do jogo oculto (`xvfb-run` no linux, ou argumentos silenciosos) apenas pelo tempo suficiente para o Python rodar e gerar um log contendo os textos (`dump.txt`).
3. **Parse e Filtro:** O arquivo `dump.txt` é lido pelo Rust. São aplicadas regras complexas para proteger marcações `{b}...{/b}` e variáveis de script `[player_name]`. Isso é feito substituindo-os por marcadores numéricos temporários (ex: `777001777`) antes de mandar para a API.
4. **Tradução:** Blocos de texto são despachados para a `api.rs`, usando multi-threading ou em lotes sequenciais dependendo da configuração.
5. **Reconstrução:** Após a tradução, as marcações numéricas são convertidas de volta para as originais.
6. **Deploy:** São gerados os arquivos de tradução originais do Ren'Py (`.rpy` traduzidos), que o jogo compilará na próxima inicialização, garantindo compatibilidade exata de chaves (`old "..."`).

### 4. Correção de Fonte (Font Injector)
Em jogos americanos ou japoneses, a fonte nativa não sabe desenhar "ã", "é", "ç", etc., fazendo as letras ficarem invisíveis.
O módulo `font_injector.rs` copia uma fonte do computador do usuário para a raiz do jogo, criando um patch `.rpy` (no Ren'Py) ou manipulando o `Config.ini` (no Unity) para forçar as interfaces gráficas a substituírem a família de fonte. O GTK4 possui suporte a CSS dinâmico (`@font-face`), então esse módulo consegue mostrar na hora para o usuário um Preview da fonte, antes mesmo dele injetar no jogo.

---

## Como usamos a Biblioteca GTK (gtk4-rs)

A construção da UI é 100% declarativa e reativa via closures de Rust:

### Widgets e Hierarquia
Em vez de definir XMLs separados, os containers são aninhados em Rust. Exemplo, a janela recebe um `Box` vertical, que recebe o `Stack` e o `HeaderBar` customizado.
```rust
let root = Box::new(Orientation::Vertical, 0);
let title_bar = Box::new(Orientation::Horizontal, 0);
root.append(&title_bar);
window.set_child(Some(&root));
```

### Estilização via CSS (CssProvider)
Todas as formatações (cores, bordas arredondadas, sombras, efeitos de hover) são definidas em arquivos `.css` ou strings nativas, aplicadas globalmente ou por widget através do `CssProvider`.
O aplicativo carrega o arquivo `style.css` (ou usa um fallback em memória) no início:
```rust
let provider = gtk::CssProvider::new();
provider.load_from_data(include_str!("style.css"));
gtk::style_context_add_provider_for_display(
    &gdk::Display::default().unwrap(),
    &provider,
    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
);
```

### Clonando Variáveis para Closures (Callbacks)
A maior dificuldade no desenvolvimento com `gtk-rs` em Rust são as regras rígidas de ownership (empréstimo de variáveis).
Para lidar com botões alterando outros elementos da UI, usamos **Clones Inteligentes** (Referência Contada com RC).
```rust
let input_caixa = Entry::new();
let botao = Button::new();

// Precisamos clonar as referências da memória antes de jogar no move closure
let ic = input_caixa.clone(); 
botao.connect_clicked(move |_| {
    ic.set_text("Botão clicado!");
});
```

### Multi-threading Seguro (Channels)
GTK roda numa "Main Loop" estrita. Você não pode atualizar uma barra de progresso ou caixa de log a partir de uma Thread que faz requests pesados (isso cracharia a UI ou geraria comportamentos estranhos em C).
Para extrair um jogo em Background (assíncrono/paralelo) sem congelar o app, o fluxo usado é:
1. Nós criamos um canal nativo do GTK, `glib::MainContext::channel`.
2. A UI obtém o *Receiver* (rx), e dispara o processo de Extração numa Thread separada (passando o *Sender* tx).
3. A *Background Thread* usa `tx.send(Mensagem)` enviando dados (string de logs ou números inteiros para progresso).
4. O `rx.attach()` rodando na Main Thread do GTK escuta esses dados, e só ele muda as Labels ou Progress Bars nativamente.
Esse fluxo garante performance multi-thread, proteção segura de memória, e uma interface 100% fluida sem travar.
