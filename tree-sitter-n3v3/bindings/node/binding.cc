#include <napi.h>
#include "tree_sitter/api.h"

extern "C" TSLanguage *tree_sitter_n3v3();

Napi::Object Init(Napi::Env env, Napi::Object exports) {
    exports["language"] = Napi::External<TSLanguage>::New(env, tree_sitter_n3v3());
    return exports;
}

NODE_API_MODULE(tree_sitter_n3v3_binding, Init)
