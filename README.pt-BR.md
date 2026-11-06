<p align="center">
  <img src="app/src-tauri/app-icon.png" width="96" alt="Ícone do Subquest" />
</p>

# Subquest: tradutor de jogos em tempo real

Jogue qualquer jogo no seu idioma. O Subquest lê os diálogos da tela, traduz com IA que entende o contexto e mostra a tradução por cima do jogo, sem roubar o foco.

> 🇺🇸 [Read in English](README.md)

**Situação:** primeira versão de testes. Captura, OCR, tradução em duas camadas, a tradução por cima do jogo com as fontes do próprio jogo, a barrinha e os atalhos já funcionam de ponta a ponta; veja o [roteiro](#roteiro) para os próximos passos.

## Instalar

1. Baixe o `Subquest_x.y.z_x64-setup.exe` na página de [Releases](https://github.com/The-Jonas/Subquest/releases) e execute. O Windows pode avisar que o editor é desconhecido (o instalador ainda não é assinado): clique em **Mais informações → Executar assim mesmo**.
2. Abra o Subquest. Um tour guiado mostra cada tela; dá para abrir de novo quando quiser pelo menu.
3. Em **Ajustes → Tradução**, cole uma chave de API do tradutor. O [Google AI Studio](https://aistudio.google.com/apikey) dá uma chave grátis do Gemini; ela fica só no seu computador.
4. Deixe o jogo **em janela ou janela sem borda** (a tela cheia exclusiva bloqueia a captura), escolha ele em **Jogar**, marque onde os textos aparecem e comece.

Precisa do Windows 10 (versão 1903 ou mais nova) ou Windows 11. Jogos em japonês, chinês ou coreano também precisam do pacote desse idioma instalado no Windows (Configurações → Hora e idioma → Idioma).

## Por que mais um tradutor?

As ferramentas que existem funcionam, mas costumam ser lentas, traduzem cada fala isolada e enchem o usuário de opções. O Subquest foca em três coisas:

- **Velocidade.** Captura da janela pela GPU, OCR só quando o texto muda *e* para de mudar, e um pipeline que descarta quadros velhos em vez de enfileirá-los.
- **Traduções melhores.** O modelo de linguagem recebe as falas anteriores e é instruído a corrigir erros de OCR; um tradutor rápido tapa o buraco enquanto ele responde.
- **Interface limpa.** Três passos para começar: escolher o jogo, marcar a área das falas e começar.

A arquitetura completa, com o diagrama das camadas, está no [README em inglês](README.md#architecture).

## Compilar o código (Windows)

Só é preciso para mexer no código; para usar o Subquest, basta [instalar](#instalar).

Requisitos: Visual Studio 2022 (carga de trabalho C++), CMake 3.24+, Rust (stable) e Node.js 20+.

```powershell
# 1. Núcleo (também copia a DLL para app/src-tauri/bin)
cd core
cmake --preset windows-release
cmake --build --preset release
ctest --test-dir build -C Release --output-on-failure

# 2. App
cd ../app
npm install
npm run tauri dev
```

Para mexer só na interface, `npm run dev` roda no navegador com dados de exemplo, sem precisar de Rust nem C++.

### Instalador

Com o núcleo compilado (passo 1 acima, que coloca a DLL em `app/src-tauri/bin`):

```powershell
cd app
npm run tauri build
```

O instalador fica em `app/src-tauri/target/release/bundle/nsis/`. Ele leva a DLL do núcleo junto e instala o WebView2 se o PC não tiver; os runtimes de C e C++ vão embutidos, então não precisa instalar mais nada.

## Roteiro

1. ~~**MVP:** captura WGC, OCR do Windows (inglês), marcação da área, tradução com LLM, overlay e latência por etapa.~~ Pronto.
2. **Idiomas asiáticos:** PaddleOCR via ONNX Runtime + DirectML (japonês, chinês e coreano). *Não começado; o OCR do Windows já lê esses idiomas quando o pacote de idioma do Windows está instalado.*
3. **Qualidade:** ~~contexto~~, ~~cache persistente~~ (um arquivo JSON em vez de SQLite), ~~streaming~~, ~~perfis por jogo~~, ~~um segundo tradutor rápido~~, ~~estilos com as fontes do jogo~~. *Falta: o glossário por jogo.*
4. ~~**Modo tela inteira:** detecção de texto na janela toda, com a tradução desenhada no lugar.~~ Pronto.
5. **Acabamento:** ~~atalhos~~, ~~barrinha no jogo~~, ~~tour guiado~~, ~~temas~~, instalador *(primeira versão)* e *text hooking* para engines suportadas.
6. **Outras telas:** modo segunda tela para consoles e celulares, e Android.

## Licença

[GNU GPL v3.0](LICENSE). As licenças de terceiros estão em [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
