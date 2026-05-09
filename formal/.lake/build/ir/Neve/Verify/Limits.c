// Lean compiler output
// Module: Neve.Verify.Limits
// Imports: public import Init public import Neve.Spec.Effects
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
static const lean_ctor_object lp_NeveFormal_Neve_check__stdin___closed__0_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*1 + 0, .m_other = 1, .m_tag = 1}, .m_objs = {((lean_object*)(((size_t)(0) << 1) | 1))}};
static const lean_object* lp_NeveFormal_Neve_check__stdin___closed__0 = (const lean_object*)&lp_NeveFormal_Neve_check__stdin___closed__0_value;
static const lean_string_object lp_NeveFormal_Neve_check__stdin___closed__1_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 20, .m_capacity = 20, .m_length = 19, .m_data = "stdin exceeds limit"};
static const lean_object* lp_NeveFormal_Neve_check__stdin___closed__1 = (const lean_object*)&lp_NeveFormal_Neve_check__stdin___closed__1_value;
static const lean_ctor_object lp_NeveFormal_Neve_check__stdin___closed__2_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*1 + 0, .m_other = 1, .m_tag = 0}, .m_objs = {((lean_object*)&lp_NeveFormal_Neve_check__stdin___closed__1_value)}};
static const lean_object* lp_NeveFormal_Neve_check__stdin___closed__2 = (const lean_object*)&lp_NeveFormal_Neve_check__stdin___closed__2_value;
uint8_t lean_nat_dec_lt(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_Neve_check__stdin(lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_Neve_check__stdin___boxed(lean_object*);
static const lean_string_object lp_NeveFormal_Neve_check__output___closed__0_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 21, .m_capacity = 21, .m_length = 20, .m_data = "stderr exceeds limit"};
static const lean_object* lp_NeveFormal_Neve_check__output___closed__0 = (const lean_object*)&lp_NeveFormal_Neve_check__output___closed__0_value;
static const lean_ctor_object lp_NeveFormal_Neve_check__output___closed__1_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*1 + 0, .m_other = 1, .m_tag = 0}, .m_objs = {((lean_object*)&lp_NeveFormal_Neve_check__output___closed__0_value)}};
static const lean_object* lp_NeveFormal_Neve_check__output___closed__1 = (const lean_object*)&lp_NeveFormal_Neve_check__output___closed__1_value;
static const lean_string_object lp_NeveFormal_Neve_check__output___closed__2_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 21, .m_capacity = 21, .m_length = 20, .m_data = "stdout exceeds limit"};
static const lean_object* lp_NeveFormal_Neve_check__output___closed__2 = (const lean_object*)&lp_NeveFormal_Neve_check__output___closed__2_value;
static const lean_ctor_object lp_NeveFormal_Neve_check__output___closed__3_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*1 + 0, .m_other = 1, .m_tag = 0}, .m_objs = {((lean_object*)&lp_NeveFormal_Neve_check__output___closed__2_value)}};
static const lean_object* lp_NeveFormal_Neve_check__output___closed__3 = (const lean_object*)&lp_NeveFormal_Neve_check__output___closed__3_value;
LEAN_EXPORT lean_object* lp_NeveFormal_Neve_check__output(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_Neve_check__output___boxed(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_Neve_check__stdin(lean_object* x_1) {
_start:
{
lean_object* x_2; uint8_t x_3; 
x_2 = lean_unsigned_to_nat(10485760u);
x_3 = lean_nat_dec_lt(x_2, x_1);
if (x_3 == 0)
{
lean_object* x_4; 
x_4 = ((lean_object*)(lp_NeveFormal_Neve_check__stdin___closed__0));
return x_4;
}
else
{
lean_object* x_5; 
x_5 = ((lean_object*)(lp_NeveFormal_Neve_check__stdin___closed__2));
return x_5;
}
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_Neve_check__stdin___boxed(lean_object* x_1) {
_start:
{
lean_object* x_2; 
x_2 = lp_NeveFormal_Neve_check__stdin(x_1);
lean_dec(x_1);
return x_2;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_Neve_check__output(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; uint8_t x_4; 
x_3 = lean_unsigned_to_nat(52428800u);
x_4 = lean_nat_dec_lt(x_3, x_1);
if (x_4 == 0)
{
uint8_t x_5; 
x_5 = lean_nat_dec_lt(x_3, x_2);
if (x_5 == 0)
{
lean_object* x_6; 
x_6 = ((lean_object*)(lp_NeveFormal_Neve_check__stdin___closed__0));
return x_6;
}
else
{
lean_object* x_7; 
x_7 = ((lean_object*)(lp_NeveFormal_Neve_check__output___closed__1));
return x_7;
}
}
else
{
lean_object* x_8; 
x_8 = ((lean_object*)(lp_NeveFormal_Neve_check__output___closed__3));
return x_8;
}
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_Neve_check__output___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lp_NeveFormal_Neve_check__output(x_1, x_2);
lean_dec(x_2);
lean_dec(x_1);
return x_3;
}
}
lean_object* initialize_Init(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Spec_Effects(uint8_t builtin);
static bool _G_initialized = false;
LEAN_EXPORT lean_object* initialize_NeveFormal_Neve_Verify_Limits(uint8_t builtin) {
lean_object * res;
if (_G_initialized) return lean_io_result_mk_ok(lean_box(0));
_G_initialized = true;
res = initialize_Init(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Spec_Effects(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
return lean_io_result_mk_ok(lean_box(0));
}
#ifdef __cplusplus
}
#endif
