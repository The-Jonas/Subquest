<p align="center">
  <img src="app/src-tauri/app-icon.png" width="96" alt="Ícone do Subquest" />
</p>

# Subquest: tradutor de jogos em tempo real

Jogue qualquer jogo no seu idioma. O Subquest lê os diálogos da tela, traduz com IA que entende o contexto e mostra a tradução por cima do jogo, sem roubar o foco.

> 🇺🇸 [Read in English](README.md)

**Situação:** início do desenvolvimento. Captura, OCR (inglês), tradução e a legenda por cima do jogo já funcionam de ponta a ponta; veja o [roteiro](#roteiro) para os próximos passos.

## Por que mais um tradutor?

As ferramentas que existem funcionam, mas costumam ser lentas, traduzem cada fala isolada e enchem o usuário de opções. O Subquest foca em três coisas:

- **Velocidade.** Captura da janela pela GPU, OCR só quando o texto muda *e* para de mudar, e um pipeline que descarta quadros velhos em vez de enfileirá-los.
- **Traduções melhores.** O modelo de linguagem recebe as falas anteriores e um glossário por jogo, e é instruído a corrigir erros de OCR.
- **Interface limpa.** Três passos para começar: escolher o jogo, marcar a área das falas e começar.

A arquitetura completa, com o diagrama das camadas, está no [README em inglês](README.md#architecture).

## Como compilar (Windows)

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

## Roteiro

1. ~~**MVP:** captura WGC, OCR do Windows (inglês), marcação da área, tradução com LLM, overlay e latência por etapa.~~ Pronto.
2. **Idiomas asiáticos:** PaddleOCR via ONNX Runtime + DirectML (japonês, chinês e coreano).
3. **Qualidade:** contexto, glossário, cache persistente (SQLite), streaming e perfis por jogo.
4. **Modo tela inteira:** detecção de texto na janela toda, com a tradução desenhada no lugar.
5. **Acabamento:** atalhos, instalador e *text hooking* para engines suportadas.

## Licença

[GNU GPL v3.0](LICENSE). As licenças de terceiros estão em [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
