#include <napi.h>
#include "tree_sitter/api.h"

extern "C" TSLanguage *tree_sitter_neve();

Napi::Object Init(Napi::Env env, Napi::Object exports) {
    exports["language"] = Napi::External<TSLanguage>::New(env, tree_sitter_neve());
    return exports;
}

NODE_API_MODULE(tree_sitter_neve_binding, Init)
