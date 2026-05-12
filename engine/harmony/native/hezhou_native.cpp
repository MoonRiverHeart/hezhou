#include "hezhou_native.h"
#include <napi/native_api.h>
#include <native_window/external_window.h>

static void* g_engine = nullptr;

// Surface callbacks for XComponent
static void OnXComponentCreated(OH_NativeWindow* window) {
    if (!g_engine) return;
    
    void* ctx = harmony_get_window_context(g_engine);
    
    int32_t width = 0, height = 0;
    OH_NativeWindow_GetSize(window, &width, &height);
    
    harmony_on_surface_created(ctx, window, width, height);
}

static void OnXComponentSizeChanged(OH_NativeWindow* window, int32_t width, int32_t height) {
    if (!g_engine) return;
    
    void* ctx = harmony_get_window_context(g_engine);
    harmony_on_surface_resize(ctx, width, height);
}

static void OnXComponentDestroyed(OH_NativeWindow* window) {
    if (!g_engine) return;
    
    void* ctx = harmony_get_window_context(g_engine);
    harmony_on_surface_destroyed(ctx);
}

// NAPI functions
static napi_value InitEngine(napi_env env, napi_callback_info info) {
    g_engine = harmony_engine_init();
    
    napi_value result;
    napi_create_int32(env, g_engine ? 1 : 0, &result);
    return result;
}

static napi_value ShutdownEngine(napi_env env, napi_callback_info info) {
    if (g_engine) {
        harmony_engine_shutdown(g_engine);
        g_engine = nullptr;
    }
    return nullptr;
}

static napi_value OnTouchEvent(napi_env env, napi_callback_info info) {
    if (!g_engine) return nullptr;
    
    size_t argc = 5;
    napi_value args[5];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    
    int32_t action, pointer_id;
    double x, y, timestamp;
    
    napi_get_value_int32(env, args[0], &action);
    napi_get_value_double(env, args[1], &x);
    napi_get_value_double(env, args[2], &y);
    napi_get_value_double(env, args[3], &timestamp);
    napi_get_value_int32(env, args[4], &pointer_id);
    
    TouchEvent event = {
        .action = (TouchAction)action,
        .x = (float)x,
        .y = (float)y,
        .timestamp = (uint64_t)timestamp,
        .pointer_id = pointer_id,
    };
    
    void* bus = harmony_get_event_bus(g_engine);
    harmony_on_touch_event(bus, &event);
    
    return nullptr;
}

static napi_value OnKeyEvent(napi_env env, napi_callback_info info) {
    if (!g_engine) return nullptr;
    
    size_t argc = 4;
    napi_value args[4];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    
    int32_t action, keycode, modifiers;
    double timestamp;
    
    napi_get_value_int32(env, args[0], &action);
    napi_get_value_int32(env, args[1], &keycode);
    napi_get_value_int32(env, args[2], &modifiers);
    napi_get_value_double(env, args[3], &timestamp);
    
    KeyEvent event = {
        .action = (KeyAction)action,
        .keycode = keycode,
        .modifiers = modifiers,
        .timestamp = (uint64_t)timestamp,
    };
    
    void* bus = harmony_get_event_bus(g_engine);
    harmony_on_key_event(bus, &event);
    
    return nullptr;
}

static napi_value OnResize(napi_env env, napi_callback_info info) {
    if (!g_engine) return nullptr;
    
    size_t argc = 2;
    napi_value args[2];
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);
    
    int32_t width, height;
    napi_get_value_int32(env, args[0], &width);
    napi_get_value_int32(env, args[1], &height);
    
    SizeEvent event = { width, height };
    
    void* bus = harmony_get_event_bus(g_engine);
    harmony_on_size_event(bus, &event);
    
    return nullptr;
}

// NAPI module init
EXTERN_C_START
static napi_value NAPI_Init(napi_env env, napi_value exports) {
    napi_property_descriptor desc[] = {
        { "initEngine", nullptr, InitEngine, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "shutdownEngine", nullptr, ShutdownEngine, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "onTouchEvent", nullptr, OnTouchEvent, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "onKeyEvent", nullptr, OnKeyEvent, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "onResize", nullptr, OnResize, nullptr, nullptr, nullptr, napi_default, nullptr },
    };
    
    napi_define_properties(env, exports, sizeof(desc) / sizeof(desc[0]), desc);
    return exports;
}
EXTERN_C_END

// NAPI module registration
static napi_module module = {
    .nm_version = 1,
    .nm_flags = 0,
    .nm_filename = nullptr,
    .nm_register_func = NAPI_Init,
    .nm_modname = "hezhou_native",
    .nm_priv = nullptr,
    .reserved = { 0 },
};

extern __attribute__((visibility("default"))) napi_module* NAPI_GetModule(void) {
    return &module;
}