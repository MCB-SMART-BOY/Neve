// Lean compiler output
// Module: Neve
// Imports: public import Init public import Neve.Spec.Syntax public import Neve.Spec.Typing public import Neve.Spec.Eval public import Neve.Spec.Effects public import Neve.Proofs.Values public import Neve.Proofs.Context public import Neve.Proofs.Safety public import Neve.Verify.Path public import Neve.Verify.Environ public import Neve.Verify.Limits public import Neve.Tests.Eval
#include <lean/lean.h>
#if defined(__clang__)
#pragma clang diagnostic ignored "-Wunused-parameter"
#pragma clang diagnostic ignored "-Wunused-label"
#elif defined(__GNUC__) && !defined(__CLANG__)
#pragma GCC diagnostic ignored "-Wunused-parameter"
#pragma GCC diagnostic ignored "-Wunused-label"
#pragma GCC diagnostic ignored "-Wunused-but-set-variable"
#endif
#ifdef __cplusplus
extern "C" {
#endif
lean_object* initialize_Init(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Spec_Syntax(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Spec_Typing(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Spec_Eval(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Spec_Effects(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Proofs_Values(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Proofs_Context(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Proofs_Safety(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Verify_Path(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Verify_Environ(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Verify_Limits(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Tests_Eval(uint8_t builtin);
static bool _G_initialized = false;
LEAN_EXPORT lean_object* initialize_NeveFormal_Neve(uint8_t builtin) {
lean_object * res;
if (_G_initialized) return lean_io_result_mk_ok(lean_box(0));
_G_initialized = true;
res = initialize_Init(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Spec_Syntax(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Spec_Typing(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Spec_Eval(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Spec_Effects(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Proofs_Values(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Proofs_Context(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Proofs_Safety(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Verify_Path(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Verify_Environ(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Verify_Limits(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Tests_Eval(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
return lean_io_result_mk_ok(lean_box(0));
}
#ifdef __cplusplus
}
#endif
