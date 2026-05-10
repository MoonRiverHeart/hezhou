#include "napi/native_api.h"

// 声明 Rust 导出的 C 接口
extern "C" {
    int add(int a, int b);
    int multiply(int a, int b);
    char* get_message();
    void free_message(char* ptr);
}

// NAPI: add(a, b)
static napi_value NapiAdd(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    int32_t a = 0, b = 0;
    napi_get_value_int32(env, args[0], &a);
    napi_get_value_int32(env, args[1], &b);

    napi_value result;
    napi_create_int32(env, add(a, b), &result);
    return result;
}

// NAPI: multiply(a, b)
static napi_value NapiMultiply(napi_env env, napi_callback_info info) {
    size_t argc = 2;
    napi_value args[2] = {nullptr};
    napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

    int32_t a = 0, b = 0;
    napi_get_value_int32(env, args[0], &a);
    napi_get_value_int32(env, args[1], &b);

    napi_value result;
    napi_create_int32(env, multiply(a, b), &result);
    return result;
}

// NAPI: getMessage()
static napi_value NapiGetMessage(napi_env env, napi_callback_info info) {
    char* msg = get_message();
    napi_value result;
    napi_create_string_utf8(env, msg, NAPI_AUTO_LENGTH, &result);
    // 调用后立即释放 Rust 分配的内存，防止泄漏
    free_message(msg);
    return result;
}

// 模块初始化：注册导出到 ArkTS 的函数
static napi_value Init(napi_env env, napi_value exports) {
    napi_property_descriptor desc[] = {
        { "add", nullptr, NapiAdd, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "multiply", nullptr, NapiMultiply, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "getMessage", nullptr, NapiGetMessage, nullptr, nullptr, nullptr, napi_default, nullptr }
    };
    napi_define_properties(env, exports, sizeof(desc) / sizeof(desc[0]), desc);
    return exports;
}

// 注册 NAPI 模块，模块名为 "entry"
NAPI_MODULE(entry, Init)
