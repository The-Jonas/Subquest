/*
 * core_api.h — API pública em C da subquest_core.dll.
 *
 * Esta é a ÚNICA fronteira entre o núcleo em C++ e o app em Rust (Tauri).
 * Só tipos de C aqui: nada de std::string, exceções ou classes atravessam a DLL.
 * Toda string é UTF-8 terminada em '\0'.
 */
#ifndef SUBQUEST_CORE_API_H
#define SUBQUEST_CORE_API_H

#include <stdint.h>

#ifdef _WIN32
  #ifdef SUBQUEST_CORE_BUILD
    #define SQ_API __declspec(dllexport)
  #else
    #define SQ_API __declspec(dllimport)
  #endif
#else
  #define SQ_API __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define SQ_API_VERSION 1u /* incrementar sempre que a ABI mudar */

typedef enum SqStatus {
    SQ_OK = 0,
    SQ_ERR_INVALID_ARG = 1,     /* ponteiro nulo ou valor fora do intervalo */
    SQ_ERR_ALREADY_RUNNING = 2, /* sq_start chamado com sessão ativa */
    SQ_ERR_NOT_RUNNING = 3,     /* sq_stop sem sessão ativa */
    SQ_ERR_PLATFORM = 4,        /* falha numa API do Windows (captura, OCR...) */
    SQ_ERR_NOT_IMPLEMENTED = 5  /* recurso ainda em stub */
} SqStatus;

typedef struct SqRect {
    int32_t x;      /* relativo ao canto superior esquerdo da área cliente da janela */
    int32_t y;
    int32_t width;
    int32_t height;
} SqRect;

typedef struct SqSessionConfig {
    uint64_t window_handle;      /* HWND do jogo, convertido para inteiro */
    SqRect region;               /* área do diálogo; width/height = 0 => janela inteira */
    const char* ocr_language;    /* tag BCP-47: "en", "ja", "zh-Hans", "ko" */
    uint32_t settle_frames;      /* quadros iguais seguidos antes de rodar o OCR (padrão 3) */
    float change_threshold;      /* diferença média mínima (0..1) para contar como mudança (padrão 0.02) */
} SqSessionConfig;

/* Chamado na thread de OCR quando um texto NOVO é reconhecido. */
typedef void (*SqTextCallback)(const char* utf8_text, uint64_t frame_id, double ocr_ms, void* user_data);

/* Chamado uma vez por janela visível durante sq_list_windows. */
typedef void (*SqWindowCallback)(uint64_t window_handle, const char* utf8_title, const char* utf8_process, void* user_data);

SQ_API uint32_t sq_api_version(void);                                    /* versão da ABI, para o Rust checar compatibilidade */
SQ_API SqStatus sq_list_windows(SqWindowCallback callback, void* user_data); /* enumera janelas de nível superior visíveis */
SQ_API SqStatus sq_start(const SqSessionConfig* config, SqTextCallback callback, void* user_data); /* inicia captura + OCR */
SQ_API SqStatus sq_stop(void);                                            /* para a sessão e espera as threads terminarem */
SQ_API SqStatus sq_overlay_set_text(const char* utf8_text);               /* troca o texto exibido no overlay */
SQ_API SqStatus sq_overlay_set_visible(int32_t visible);                  /* mostra (1) ou esconde (0) o overlay */
SQ_API const char* sq_last_error(void);                                   /* mensagem do último erro (válida até a próxima chamada) */

#ifdef __cplusplus
}
#endif

#endif /* SUBQUEST_CORE_API_H */
