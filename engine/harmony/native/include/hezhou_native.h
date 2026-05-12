#ifndef HEZHOU_NATIVE_H
#define HEZHOU_NATIVE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Event types
typedef enum {
    TOUCH_ACTION_DOWN = 0,
    TOUCH_ACTION_MOVE = 1,
    TOUCH_ACTION_UP = 2,
    TOUCH_ACTION_CANCEL = 3,
} TouchAction;

typedef struct {
    TouchAction action;
    float x;
    float y;
    uint64_t timestamp;
    int32_t pointer_id;
} TouchEvent;

typedef enum {
    KEY_ACTION_PRESS = 0,
    KEY_ACTION_RELEASE = 1,
} KeyAction;

typedef struct {
    KeyAction action;
    int32_t keycode;
    int32_t modifiers;
    uint64_t timestamp;
} KeyEvent;

typedef struct {
    int32_t width;
    int32_t height;
} SizeEvent;

typedef enum {
    LIFECYCLE_CREATE = 0,
    LIFECYCLE_START = 1,
    LIFECYCLE_RESUME = 2,
    LIFECYCLE_PAUSE = 3,
    LIFECYCLE_STOP = 4,
    LIFECYCLE_DESTROY = 5,
} LifecycleState;

typedef struct {
    LifecycleState state;
} LifecycleEvent;

// Engine lifecycle
void* harmony_engine_init(void);
void harmony_engine_shutdown(void* engine);

// Window context
void* harmony_get_window_context(void* engine);
void* harmony_get_event_bus(void* engine);

// Surface callbacks (called by HarmonyOS XComponent)
void harmony_on_surface_created(void* ctx, void* window, int32_t width, int32_t height);
void harmony_on_surface_resize(void* ctx, int32_t width, int32_t height);
void harmony_on_surface_destroyed(void* ctx);

// Event callbacks (called by NAPI)
void harmony_on_touch_event(void* bus, const TouchEvent* event);
void harmony_on_key_event(void* bus, const KeyEvent* event);
void harmony_on_size_event(void* bus, const SizeEvent* event);
void harmony_on_lifecycle_event(void* bus, const LifecycleEvent* event);

#ifdef __cplusplus
}
#endif

#endif // HEZHOU_NATIVE_H