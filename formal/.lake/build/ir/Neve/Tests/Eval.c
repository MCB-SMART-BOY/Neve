// Lean compiler output
// Module: Neve.Tests.Eval
// Imports: public import Init public import Neve.Spec.Syntax
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
uint8_t lean_int_dec_eq(lean_object*, lean_object*);
uint8_t lean_string_dec_eq(lean_object*, lean_object*);
LEAN_EXPORT uint8_t lp_NeveFormal_matchesPattern(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_matchesPattern___boxed(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_findArm(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_findArm___boxed(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_List_lookup___at___00eval_spec__0___redArg(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_List_lookup___at___00eval_spec__0___redArg___boxed(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_eval(lean_object*, lean_object*);
lean_object* lean_int_add(lean_object*, lean_object*);
lean_object* lean_int_sub(lean_object*, lean_object*);
lean_object* lean_int_mul(lean_object*, lean_object*);
uint8_t lp_NeveFormal_Neve_instBEqValue_beq(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_List_lookup___at___00eval_spec__0(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_List_lookup___at___00eval_spec__0___boxed(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_evalClosed(lean_object*);
lean_object* lean_nat_to_int(lean_object*);
static lean_once_cell_t lp_NeveFormal_fmt___closed__0_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_fmt___closed__0;
static const lean_string_object lp_NeveFormal_fmt___closed__1_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 2, .m_capacity = 2, .m_length = 1, .m_data = "-"};
static const lean_object* lp_NeveFormal_fmt___closed__1 = (const lean_object*)&lp_NeveFormal_fmt___closed__1_value;
static const lean_string_object lp_NeveFormal_fmt___closed__2_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 6, .m_capacity = 6, .m_length = 5, .m_data = "false"};
static const lean_object* lp_NeveFormal_fmt___closed__2 = (const lean_object*)&lp_NeveFormal_fmt___closed__2_value;
static const lean_string_object lp_NeveFormal_fmt___closed__3_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 5, .m_capacity = 5, .m_length = 4, .m_data = "true"};
static const lean_object* lp_NeveFormal_fmt___closed__3 = (const lean_object*)&lp_NeveFormal_fmt___closed__3_value;
static const lean_string_object lp_NeveFormal_fmt___closed__4_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 3, .m_capacity = 3, .m_length = 2, .m_data = "()"};
static const lean_object* lp_NeveFormal_fmt___closed__4 = (const lean_object*)&lp_NeveFormal_fmt___closed__4_value;
static const lean_string_object lp_NeveFormal_fmt___closed__5_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 2, .m_capacity = 2, .m_length = 1, .m_data = "\""};
static const lean_object* lp_NeveFormal_fmt___closed__5 = (const lean_object*)&lp_NeveFormal_fmt___closed__5_value;
static const lean_string_object lp_NeveFormal_fmt___closed__6_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 10, .m_capacity = 10, .m_length = 9, .m_data = "<complex>"};
static const lean_object* lp_NeveFormal_fmt___closed__6 = (const lean_object*)&lp_NeveFormal_fmt___closed__6_value;
uint8_t lean_int_dec_lt(lean_object*, lean_object*);
lean_object* lean_nat_abs(lean_object*);
lean_object* l_Nat_reprFast(lean_object*);
lean_object* lean_nat_sub(lean_object*, lean_object*);
lean_object* lean_nat_add(lean_object*, lean_object*);
lean_object* lean_string_append(lean_object*, lean_object*);
lean_object* lean_string_append(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_fmt(lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_fmt___boxed(lean_object*);
lean_object* lean_get_stdout();
LEAN_EXPORT lean_object* lp_NeveFormal_IO_print___at___00IO_println___at___00runTest_spec__0_spec__0(lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_IO_print___at___00IO_println___at___00runTest_spec__0_spec__0___boxed(lean_object*, lean_object*);
lean_object* lean_string_push(lean_object*, uint32_t);
LEAN_EXPORT lean_object* lp_NeveFormal_IO_println___at___00runTest_spec__0(lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_IO_println___at___00runTest_spec__0___boxed(lean_object*, lean_object*);
static const lean_string_object lp_NeveFormal_runTest___closed__0_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 5, .m_capacity = 5, .m_length = 2, .m_data = "❌ "};
static const lean_object* lp_NeveFormal_runTest___closed__0 = (const lean_object*)&lp_NeveFormal_runTest___closed__0_value;
static const lean_string_object lp_NeveFormal_runTest___closed__1_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 7, .m_capacity = 7, .m_length = 6, .m_data = ": got "};
static const lean_object* lp_NeveFormal_runTest___closed__1 = (const lean_object*)&lp_NeveFormal_runTest___closed__1_value;
static const lean_string_object lp_NeveFormal_runTest___closed__2_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 12, .m_capacity = 12, .m_length = 11, .m_data = ", expected "};
static const lean_object* lp_NeveFormal_runTest___closed__2 = (const lean_object*)&lp_NeveFormal_runTest___closed__2_value;
static const lean_string_object lp_NeveFormal_runTest___closed__3_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 5, .m_capacity = 5, .m_length = 2, .m_data = "✅ "};
static const lean_object* lp_NeveFormal_runTest___closed__3 = (const lean_object*)&lp_NeveFormal_runTest___closed__3_value;
static const lean_string_object lp_NeveFormal_runTest___closed__4_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 4, .m_capacity = 4, .m_length = 3, .m_data = " = "};
static const lean_object* lp_NeveFormal_runTest___closed__4 = (const lean_object*)&lp_NeveFormal_runTest___closed__4_value;
LEAN_EXPORT lean_object* lp_NeveFormal_runTest(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_NeveFormal_runTest___boxed(lean_object*, lean_object*, lean_object*, lean_object*);
static const lean_string_object lp_NeveFormal_main___closed__0_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 4, .m_capacity = 4, .m_length = 3, .m_data = "1+2"};
static const lean_object* lp_NeveFormal_main___closed__0 = (const lean_object*)&lp_NeveFormal_main___closed__0_value;
static lean_once_cell_t lp_NeveFormal_main___closed__1_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__1;
static lean_once_cell_t lp_NeveFormal_main___closed__2_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__2;
static lean_once_cell_t lp_NeveFormal_main___closed__3_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__3;
static lean_once_cell_t lp_NeveFormal_main___closed__4_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__4;
static lean_once_cell_t lp_NeveFormal_main___closed__5_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__5;
static const lean_string_object lp_NeveFormal_main___closed__6_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 2, .m_capacity = 2, .m_length = 1, .m_data = "3"};
static const lean_object* lp_NeveFormal_main___closed__6 = (const lean_object*)&lp_NeveFormal_main___closed__6_value;
static const lean_string_object lp_NeveFormal_main___closed__7_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 8, .m_capacity = 8, .m_length = 7, .m_data = "(3+4)*2"};
static const lean_object* lp_NeveFormal_main___closed__7 = (const lean_object*)&lp_NeveFormal_main___closed__7_value;
static lean_once_cell_t lp_NeveFormal_main___closed__8_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__8;
static lean_once_cell_t lp_NeveFormal_main___closed__9_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__9;
static lean_once_cell_t lp_NeveFormal_main___closed__10_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__10;
static lean_once_cell_t lp_NeveFormal_main___closed__11_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__11;
static lean_once_cell_t lp_NeveFormal_main___closed__12_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__12;
static lean_once_cell_t lp_NeveFormal_main___closed__13_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__13;
static const lean_string_object lp_NeveFormal_main___closed__14_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 3, .m_capacity = 3, .m_length = 2, .m_data = "14"};
static const lean_object* lp_NeveFormal_main___closed__14 = (const lean_object*)&lp_NeveFormal_main___closed__14_value;
static const lean_string_object lp_NeveFormal_main___closed__15_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 5, .m_capacity = 5, .m_length = 4, .m_data = "10-3"};
static const lean_object* lp_NeveFormal_main___closed__15 = (const lean_object*)&lp_NeveFormal_main___closed__15_value;
static lean_once_cell_t lp_NeveFormal_main___closed__16_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__16;
static lean_once_cell_t lp_NeveFormal_main___closed__17_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__17;
static lean_once_cell_t lp_NeveFormal_main___closed__18_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__18;
static const lean_string_object lp_NeveFormal_main___closed__19_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 2, .m_capacity = 2, .m_length = 1, .m_data = "7"};
static const lean_object* lp_NeveFormal_main___closed__19 = (const lean_object*)&lp_NeveFormal_main___closed__19_value;
static const lean_string_object lp_NeveFormal_main___closed__20_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 17, .m_capacity = 17, .m_length = 16, .m_data = "let x=10+20; x*2"};
static const lean_object* lp_NeveFormal_main___closed__20 = (const lean_object*)&lp_NeveFormal_main___closed__20_value;
static const lean_string_object lp_NeveFormal_main___closed__21_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 2, .m_capacity = 2, .m_length = 1, .m_data = "x"};
static const lean_object* lp_NeveFormal_main___closed__21 = (const lean_object*)&lp_NeveFormal_main___closed__21_value;
static lean_once_cell_t lp_NeveFormal_main___closed__22_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__22;
static lean_once_cell_t lp_NeveFormal_main___closed__23_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__23;
static lean_once_cell_t lp_NeveFormal_main___closed__24_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__24;
static const lean_ctor_object lp_NeveFormal_main___closed__25_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*1 + 0, .m_other = 1, .m_tag = 6}, .m_objs = {((lean_object*)&lp_NeveFormal_main___closed__21_value)}};
static const lean_object* lp_NeveFormal_main___closed__25 = (const lean_object*)&lp_NeveFormal_main___closed__25_value;
static lean_once_cell_t lp_NeveFormal_main___closed__26_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__26;
static lean_once_cell_t lp_NeveFormal_main___closed__27_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__27;
static const lean_string_object lp_NeveFormal_main___closed__28_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 3, .m_capacity = 3, .m_length = 2, .m_data = "60"};
static const lean_object* lp_NeveFormal_main___closed__28 = (const lean_object*)&lp_NeveFormal_main___closed__28_value;
static const lean_string_object lp_NeveFormal_main___closed__29_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 16, .m_capacity = 16, .m_length = 15, .m_data = "(fn x=>x+1)(41)"};
static const lean_object* lp_NeveFormal_main___closed__29 = (const lean_object*)&lp_NeveFormal_main___closed__29_value;
static lean_once_cell_t lp_NeveFormal_main___closed__30_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__30;
static lean_once_cell_t lp_NeveFormal_main___closed__31_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__31;
static lean_once_cell_t lp_NeveFormal_main___closed__32_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__32;
static lean_once_cell_t lp_NeveFormal_main___closed__33_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__33;
static lean_once_cell_t lp_NeveFormal_main___closed__34_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__34;
static const lean_string_object lp_NeveFormal_main___closed__35_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 3, .m_capacity = 3, .m_length = 2, .m_data = "42"};
static const lean_object* lp_NeveFormal_main___closed__35 = (const lean_object*)&lp_NeveFormal_main___closed__35_value;
static const lean_string_object lp_NeveFormal_main___closed__36_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 5, .m_capacity = 5, .m_length = 4, .m_data = "1==1"};
static const lean_object* lp_NeveFormal_main___closed__36 = (const lean_object*)&lp_NeveFormal_main___closed__36_value;
static lean_once_cell_t lp_NeveFormal_main___closed__37_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__37;
static const lean_string_object lp_NeveFormal_main___closed__38_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 5, .m_capacity = 5, .m_length = 4, .m_data = "1==2"};
static const lean_object* lp_NeveFormal_main___closed__38 = (const lean_object*)&lp_NeveFormal_main___closed__38_value;
static lean_once_cell_t lp_NeveFormal_main___closed__39_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__39;
static const lean_string_object lp_NeveFormal_main___closed__40_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 12, .m_capacity = 12, .m_length = 11, .m_data = "true&&false"};
static const lean_object* lp_NeveFormal_main___closed__40 = (const lean_object*)&lp_NeveFormal_main___closed__40_value;
static const lean_ctor_object lp_NeveFormal_main___closed__41_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*0 + 8, .m_other = 0, .m_tag = 2}, .m_objs = {LEAN_SCALAR_PTR_LITERAL(1, 0, 0, 0, 0, 0, 0, 0)}};
static const lean_object* lp_NeveFormal_main___closed__41 = (const lean_object*)&lp_NeveFormal_main___closed__41_value;
static const lean_ctor_object lp_NeveFormal_main___closed__42_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*0 + 8, .m_other = 0, .m_tag = 2}, .m_objs = {LEAN_SCALAR_PTR_LITERAL(0, 0, 0, 0, 0, 0, 0, 0)}};
static const lean_object* lp_NeveFormal_main___closed__42 = (const lean_object*)&lp_NeveFormal_main___closed__42_value;
static const lean_ctor_object lp_NeveFormal_main___closed__43_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*2 + 8, .m_other = 2, .m_tag = 10}, .m_objs = {((lean_object*)&lp_NeveFormal_main___closed__41_value),((lean_object*)&lp_NeveFormal_main___closed__42_value),LEAN_SCALAR_PTR_LITERAL(11, 0, 0, 0, 0, 0, 0, 0)}};
static const lean_object* lp_NeveFormal_main___closed__43 = (const lean_object*)&lp_NeveFormal_main___closed__43_value;
static const lean_string_object lp_NeveFormal_main___closed__44_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 12, .m_capacity = 12, .m_length = 11, .m_data = "false||true"};
static const lean_object* lp_NeveFormal_main___closed__44 = (const lean_object*)&lp_NeveFormal_main___closed__44_value;
static const lean_ctor_object lp_NeveFormal_main___closed__45_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*2 + 8, .m_other = 2, .m_tag = 10}, .m_objs = {((lean_object*)&lp_NeveFormal_main___closed__42_value),((lean_object*)&lp_NeveFormal_main___closed__41_value),LEAN_SCALAR_PTR_LITERAL(12, 0, 0, 0, 0, 0, 0, 0)}};
static const lean_object* lp_NeveFormal_main___closed__45 = (const lean_object*)&lp_NeveFormal_main___closed__45_value;
static const lean_string_object lp_NeveFormal_main___closed__46_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 22, .m_capacity = 22, .m_length = 21, .m_data = "match 42 { _ => 100 }"};
static const lean_object* lp_NeveFormal_main___closed__46 = (const lean_object*)&lp_NeveFormal_main___closed__46_value;
static lean_once_cell_t lp_NeveFormal_main___closed__47_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__47;
static lean_once_cell_t lp_NeveFormal_main___closed__48_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__48;
static lean_once_cell_t lp_NeveFormal_main___closed__49_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__49;
static lean_once_cell_t lp_NeveFormal_main___closed__50_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__50;
static lean_once_cell_t lp_NeveFormal_main___closed__51_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__51;
static lean_once_cell_t lp_NeveFormal_main___closed__52_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__52;
static lean_once_cell_t lp_NeveFormal_main___closed__53_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__53;
static const lean_string_object lp_NeveFormal_main___closed__54_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 4, .m_capacity = 4, .m_length = 3, .m_data = "100"};
static const lean_object* lp_NeveFormal_main___closed__54 = (const lean_object*)&lp_NeveFormal_main___closed__54_value;
static const lean_string_object lp_NeveFormal_main___closed__55_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 37, .m_capacity = 37, .m_length = 36, .m_data = "match true { true => 1; false => 0 }"};
static const lean_object* lp_NeveFormal_main___closed__55 = (const lean_object*)&lp_NeveFormal_main___closed__55_value;
static const lean_ctor_object lp_NeveFormal_main___closed__56_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*0 + 8, .m_other = 0, .m_tag = 3}, .m_objs = {LEAN_SCALAR_PTR_LITERAL(1, 0, 0, 0, 0, 0, 0, 0)}};
static const lean_object* lp_NeveFormal_main___closed__56 = (const lean_object*)&lp_NeveFormal_main___closed__56_value;
static lean_once_cell_t lp_NeveFormal_main___closed__57_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__57;
static const lean_ctor_object lp_NeveFormal_main___closed__58_value = {.m_header = {.m_rc = 0, .m_cs_sz = sizeof(lean_ctor_object) + sizeof(void*)*0 + 8, .m_other = 0, .m_tag = 3}, .m_objs = {LEAN_SCALAR_PTR_LITERAL(0, 0, 0, 0, 0, 0, 0, 0)}};
static const lean_object* lp_NeveFormal_main___closed__58 = (const lean_object*)&lp_NeveFormal_main___closed__58_value;
static lean_once_cell_t lp_NeveFormal_main___closed__59_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__59;
static lean_once_cell_t lp_NeveFormal_main___closed__60_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__60;
static lean_once_cell_t lp_NeveFormal_main___closed__61_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__61;
static lean_once_cell_t lp_NeveFormal_main___closed__62_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__62;
static lean_once_cell_t lp_NeveFormal_main___closed__63_once = LEAN_ONCE_CELL_INITIALIZER;
static lean_object* lp_NeveFormal_main___closed__63;
static const lean_string_object lp_NeveFormal_main___closed__64_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 2, .m_capacity = 2, .m_length = 1, .m_data = "1"};
static const lean_object* lp_NeveFormal_main___closed__64 = (const lean_object*)&lp_NeveFormal_main___closed__64_value;
static const lean_string_object lp_NeveFormal_main___closed__65_value = {.m_header = {.m_rc = 0, .m_cs_sz = 0, .m_other = 0, .m_tag = 249}, .m_size = 36, .m_capacity = 36, .m_length = 35, .m_data = "\nAll Lean evaluator tests complete."};
static const lean_object* lp_NeveFormal_main___closed__65 = (const lean_object*)&lp_NeveFormal_main___closed__65_value;
LEAN_EXPORT lean_object* _lean_main();
LEAN_EXPORT lean_object* lp_NeveFormal_main___boxed(lean_object*);
LEAN_EXPORT uint8_t lp_NeveFormal_matchesPattern(lean_object* x_1, lean_object* x_2) {
_start:
{
switch (lean_obj_tag(x_1)) {
case 0:
{
uint8_t x_3; 
x_3 = 1;
return x_3;
}
case 1:
{
uint8_t x_4; 
x_4 = 1;
return x_4;
}
case 2:
{
if (lean_obj_tag(x_2) == 0)
{
lean_object* x_5; lean_object* x_6; uint8_t x_7; 
x_5 = lean_ctor_get(x_1, 0);
x_6 = lean_ctor_get(x_2, 0);
x_7 = lean_int_dec_eq(x_5, x_6);
return x_7;
}
else
{
uint8_t x_8; 
x_8 = 0;
return x_8;
}
}
case 3:
{
if (lean_obj_tag(x_2) == 2)
{
uint8_t x_9; 
x_9 = lean_ctor_get_uint8(x_1, 0);
if (x_9 == 0)
{
uint8_t x_10; 
x_10 = lean_ctor_get_uint8(x_2, 0);
if (x_10 == 0)
{
uint8_t x_11; 
x_11 = 1;
return x_11;
}
else
{
return x_9;
}
}
else
{
uint8_t x_12; 
x_12 = lean_ctor_get_uint8(x_2, 0);
return x_12;
}
}
else
{
uint8_t x_13; 
x_13 = 0;
return x_13;
}
}
case 4:
{
if (lean_obj_tag(x_2) == 4)
{
lean_object* x_14; lean_object* x_15; uint8_t x_16; 
x_14 = lean_ctor_get(x_1, 0);
x_15 = lean_ctor_get(x_2, 0);
x_16 = lean_string_dec_eq(x_14, x_15);
return x_16;
}
else
{
uint8_t x_17; 
x_17 = 0;
return x_17;
}
}
default: 
{
uint8_t x_18; 
x_18 = 0;
return x_18;
}
}
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_matchesPattern___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
uint8_t x_3; lean_object* x_4; 
x_3 = lp_NeveFormal_matchesPattern(x_1, x_2);
lean_dec(x_2);
lean_dec(x_1);
x_4 = lean_box(x_3);
return x_4;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_findArm(lean_object* x_1, lean_object* x_2) {
_start:
{
if (lean_obj_tag(x_2) == 0)
{
lean_object* x_3; 
x_3 = lean_box(0);
return x_3;
}
else
{
lean_object* x_4; lean_object* x_5; lean_object* x_6; uint8_t x_7; 
x_4 = lean_ctor_get(x_2, 0);
x_5 = lean_ctor_get(x_2, 1);
x_6 = lean_ctor_get(x_4, 0);
x_7 = lp_NeveFormal_matchesPattern(x_6, x_1);
if (x_7 == 0)
{
x_2 = x_5;
goto _start;
}
else
{
lean_object* x_9; 
lean_inc(x_4);
x_9 = lean_alloc_ctor(1, 1, 0);
lean_ctor_set(x_9, 0, x_4);
return x_9;
}
}
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_findArm___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lp_NeveFormal_findArm(x_1, x_2);
lean_dec(x_2);
lean_dec(x_1);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_List_lookup___at___00eval_spec__0___redArg(lean_object* x_1, lean_object* x_2) {
_start:
{
if (lean_obj_tag(x_2) == 0)
{
lean_object* x_3; 
x_3 = lean_box(0);
return x_3;
}
else
{
lean_object* x_4; lean_object* x_5; lean_object* x_6; lean_object* x_7; uint8_t x_8; 
x_4 = lean_ctor_get(x_2, 0);
x_5 = lean_ctor_get(x_2, 1);
x_6 = lean_ctor_get(x_4, 0);
x_7 = lean_ctor_get(x_4, 1);
x_8 = lean_string_dec_eq(x_1, x_6);
if (x_8 == 0)
{
x_2 = x_5;
goto _start;
}
else
{
lean_object* x_10; 
lean_inc(x_7);
x_10 = lean_alloc_ctor(1, 1, 0);
lean_ctor_set(x_10, 0, x_7);
return x_10;
}
}
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_List_lookup___at___00eval_spec__0___redArg___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lp_NeveFormal_List_lookup___at___00eval_spec__0___redArg(x_1, x_2);
lean_dec(x_2);
lean_dec_ref(x_1);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_eval(lean_object* x_1, lean_object* x_2) {
_start:
{
switch (lean_obj_tag(x_2)) {
case 0:
{
uint8_t x_3; 
lean_dec(x_1);
x_3 = !lean_is_exclusive(x_2);
if (x_3 == 0)
{
return x_2;
}
else
{
lean_object* x_4; lean_object* x_5; 
x_4 = lean_ctor_get(x_2, 0);
lean_inc(x_4);
lean_dec(x_2);
x_5 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_5, 0, x_4);
return x_5;
}
}
case 1:
{
uint8_t x_6; 
lean_dec(x_1);
x_6 = !lean_is_exclusive(x_2);
if (x_6 == 0)
{
return x_2;
}
else
{
double x_7; lean_object* x_8; 
x_7 = lean_ctor_get_float(x_2, 0);
lean_dec(x_2);
x_8 = lean_alloc_ctor(1, 0, 8);
lean_ctor_set_float(x_8, 0, x_7);
return x_8;
}
}
case 2:
{
uint8_t x_9; 
lean_dec(x_1);
x_9 = !lean_is_exclusive(x_2);
if (x_9 == 0)
{
return x_2;
}
else
{
uint8_t x_10; lean_object* x_11; 
x_10 = lean_ctor_get_uint8(x_2, 0);
lean_dec(x_2);
x_11 = lean_alloc_ctor(2, 0, 1);
lean_ctor_set_uint8(x_11, 0, x_10);
return x_11;
}
}
case 3:
{
uint8_t x_12; 
lean_dec(x_1);
x_12 = !lean_is_exclusive(x_2);
if (x_12 == 0)
{
return x_2;
}
else
{
uint32_t x_13; lean_object* x_14; 
x_13 = lean_ctor_get_uint32(x_2, 0);
lean_dec(x_2);
x_14 = lean_alloc_ctor(3, 0, 4);
lean_ctor_set_uint32(x_14, 0, x_13);
return x_14;
}
}
case 4:
{
uint8_t x_15; 
lean_dec(x_1);
x_15 = !lean_is_exclusive(x_2);
if (x_15 == 0)
{
return x_2;
}
else
{
lean_object* x_16; lean_object* x_17; 
x_16 = lean_ctor_get(x_2, 0);
lean_inc(x_16);
lean_dec(x_2);
x_17 = lean_alloc_ctor(4, 1, 0);
lean_ctor_set(x_17, 0, x_16);
return x_17;
}
}
case 5:
{
lean_object* x_18; 
lean_dec(x_1);
x_18 = lean_box(5);
return x_18;
}
case 6:
{
lean_object* x_19; lean_object* x_20; 
x_19 = lean_ctor_get(x_2, 0);
lean_inc_ref(x_19);
lean_dec_ref(x_2);
x_20 = lp_NeveFormal_List_lookup___at___00eval_spec__0___redArg(x_19, x_1);
lean_dec(x_1);
lean_dec_ref(x_19);
if (lean_obj_tag(x_20) == 0)
{
lean_object* x_21; 
x_21 = lean_box(5);
return x_21;
}
else
{
lean_object* x_22; 
x_22 = lean_ctor_get(x_20, 0);
lean_inc(x_22);
lean_dec_ref(x_20);
return x_22;
}
}
case 8:
{
lean_object* x_23; lean_object* x_24; lean_object* x_25; 
x_23 = lean_ctor_get(x_2, 0);
lean_inc_ref(x_23);
x_24 = lean_ctor_get(x_2, 1);
lean_inc(x_24);
lean_dec_ref(x_2);
x_25 = lean_alloc_ctor(10, 3, 0);
lean_ctor_set(x_25, 0, x_23);
lean_ctor_set(x_25, 1, x_24);
lean_ctor_set(x_25, 2, x_1);
return x_25;
}
case 7:
{
uint8_t x_26; 
x_26 = !lean_is_exclusive(x_2);
if (x_26 == 0)
{
lean_object* x_27; lean_object* x_28; lean_object* x_29; 
x_27 = lean_ctor_get(x_2, 0);
x_28 = lean_ctor_get(x_2, 1);
lean_inc(x_1);
x_29 = lp_NeveFormal_eval(x_1, x_27);
if (lean_obj_tag(x_29) == 10)
{
lean_object* x_30; lean_object* x_31; lean_object* x_32; lean_object* x_33; lean_object* x_34; 
x_30 = lean_ctor_get(x_29, 0);
lean_inc_ref(x_30);
x_31 = lean_ctor_get(x_29, 1);
lean_inc(x_31);
x_32 = lean_ctor_get(x_29, 2);
lean_inc(x_32);
lean_dec_ref(x_29);
x_33 = lp_NeveFormal_eval(x_1, x_28);
lean_ctor_set_tag(x_2, 0);
lean_ctor_set(x_2, 1, x_33);
lean_ctor_set(x_2, 0, x_30);
x_34 = lean_alloc_ctor(1, 2, 0);
lean_ctor_set(x_34, 0, x_2);
lean_ctor_set(x_34, 1, x_32);
x_1 = x_34;
x_2 = x_31;
goto _start;
}
else
{
lean_object* x_36; 
lean_dec(x_29);
lean_free_object(x_2);
lean_dec(x_28);
lean_dec(x_1);
x_36 = lean_box(5);
return x_36;
}
}
else
{
lean_object* x_37; lean_object* x_38; lean_object* x_39; 
x_37 = lean_ctor_get(x_2, 0);
x_38 = lean_ctor_get(x_2, 1);
lean_inc(x_38);
lean_inc(x_37);
lean_dec(x_2);
lean_inc(x_1);
x_39 = lp_NeveFormal_eval(x_1, x_37);
if (lean_obj_tag(x_39) == 10)
{
lean_object* x_40; lean_object* x_41; lean_object* x_42; lean_object* x_43; lean_object* x_44; lean_object* x_45; 
x_40 = lean_ctor_get(x_39, 0);
lean_inc_ref(x_40);
x_41 = lean_ctor_get(x_39, 1);
lean_inc(x_41);
x_42 = lean_ctor_get(x_39, 2);
lean_inc(x_42);
lean_dec_ref(x_39);
x_43 = lp_NeveFormal_eval(x_1, x_38);
x_44 = lean_alloc_ctor(0, 2, 0);
lean_ctor_set(x_44, 0, x_40);
lean_ctor_set(x_44, 1, x_43);
x_45 = lean_alloc_ctor(1, 2, 0);
lean_ctor_set(x_45, 0, x_44);
lean_ctor_set(x_45, 1, x_42);
x_1 = x_45;
x_2 = x_41;
goto _start;
}
else
{
lean_object* x_47; 
lean_dec(x_39);
lean_dec(x_38);
lean_dec(x_1);
x_47 = lean_box(5);
return x_47;
}
}
}
case 9:
{
lean_object* x_48; lean_object* x_49; lean_object* x_50; lean_object* x_51; lean_object* x_52; lean_object* x_53; 
x_48 = lean_ctor_get(x_2, 0);
lean_inc_ref(x_48);
x_49 = lean_ctor_get(x_2, 1);
lean_inc(x_49);
x_50 = lean_ctor_get(x_2, 2);
lean_inc(x_50);
lean_dec_ref(x_2);
lean_inc(x_1);
x_51 = lp_NeveFormal_eval(x_1, x_49);
x_52 = lean_alloc_ctor(0, 2, 0);
lean_ctor_set(x_52, 0, x_48);
lean_ctor_set(x_52, 1, x_51);
x_53 = lean_alloc_ctor(1, 2, 0);
lean_ctor_set(x_53, 0, x_52);
lean_ctor_set(x_53, 1, x_1);
x_1 = x_53;
x_2 = x_50;
goto _start;
}
case 10:
{
uint8_t x_55; 
x_55 = lean_ctor_get_uint8(x_2, sizeof(void*)*2);
switch (x_55) {
case 0:
{
lean_object* x_56; lean_object* x_57; lean_object* x_58; 
x_56 = lean_ctor_get(x_2, 0);
lean_inc(x_56);
x_57 = lean_ctor_get(x_2, 1);
lean_inc(x_57);
lean_dec_ref(x_2);
lean_inc(x_1);
x_58 = lp_NeveFormal_eval(x_1, x_56);
if (lean_obj_tag(x_58) == 0)
{
lean_object* x_59; lean_object* x_60; 
x_59 = lean_ctor_get(x_58, 0);
lean_inc(x_59);
lean_dec_ref(x_58);
x_60 = lp_NeveFormal_eval(x_1, x_57);
if (lean_obj_tag(x_60) == 0)
{
uint8_t x_61; 
x_61 = !lean_is_exclusive(x_60);
if (x_61 == 0)
{
lean_object* x_62; lean_object* x_63; 
x_62 = lean_ctor_get(x_60, 0);
x_63 = lean_int_add(x_59, x_62);
lean_dec(x_62);
lean_dec(x_59);
lean_ctor_set(x_60, 0, x_63);
return x_60;
}
else
{
lean_object* x_64; lean_object* x_65; lean_object* x_66; 
x_64 = lean_ctor_get(x_60, 0);
lean_inc(x_64);
lean_dec(x_60);
x_65 = lean_int_add(x_59, x_64);
lean_dec(x_64);
lean_dec(x_59);
x_66 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_66, 0, x_65);
return x_66;
}
}
else
{
lean_object* x_67; 
lean_dec(x_60);
lean_dec(x_59);
x_67 = lean_box(5);
return x_67;
}
}
else
{
lean_object* x_68; 
lean_dec(x_58);
lean_dec(x_57);
lean_dec(x_1);
x_68 = lean_box(5);
return x_68;
}
}
case 1:
{
lean_object* x_69; lean_object* x_70; lean_object* x_71; 
x_69 = lean_ctor_get(x_2, 0);
lean_inc(x_69);
x_70 = lean_ctor_get(x_2, 1);
lean_inc(x_70);
lean_dec_ref(x_2);
lean_inc(x_1);
x_71 = lp_NeveFormal_eval(x_1, x_69);
if (lean_obj_tag(x_71) == 0)
{
lean_object* x_72; lean_object* x_73; 
x_72 = lean_ctor_get(x_71, 0);
lean_inc(x_72);
lean_dec_ref(x_71);
x_73 = lp_NeveFormal_eval(x_1, x_70);
if (lean_obj_tag(x_73) == 0)
{
uint8_t x_74; 
x_74 = !lean_is_exclusive(x_73);
if (x_74 == 0)
{
lean_object* x_75; lean_object* x_76; 
x_75 = lean_ctor_get(x_73, 0);
x_76 = lean_int_sub(x_72, x_75);
lean_dec(x_75);
lean_dec(x_72);
lean_ctor_set(x_73, 0, x_76);
return x_73;
}
else
{
lean_object* x_77; lean_object* x_78; lean_object* x_79; 
x_77 = lean_ctor_get(x_73, 0);
lean_inc(x_77);
lean_dec(x_73);
x_78 = lean_int_sub(x_72, x_77);
lean_dec(x_77);
lean_dec(x_72);
x_79 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_79, 0, x_78);
return x_79;
}
}
else
{
lean_object* x_80; 
lean_dec(x_73);
lean_dec(x_72);
x_80 = lean_box(5);
return x_80;
}
}
else
{
lean_object* x_81; 
lean_dec(x_71);
lean_dec(x_70);
lean_dec(x_1);
x_81 = lean_box(5);
return x_81;
}
}
case 2:
{
lean_object* x_82; lean_object* x_83; lean_object* x_84; 
x_82 = lean_ctor_get(x_2, 0);
lean_inc(x_82);
x_83 = lean_ctor_get(x_2, 1);
lean_inc(x_83);
lean_dec_ref(x_2);
lean_inc(x_1);
x_84 = lp_NeveFormal_eval(x_1, x_82);
if (lean_obj_tag(x_84) == 0)
{
lean_object* x_85; lean_object* x_86; 
x_85 = lean_ctor_get(x_84, 0);
lean_inc(x_85);
lean_dec_ref(x_84);
x_86 = lp_NeveFormal_eval(x_1, x_83);
if (lean_obj_tag(x_86) == 0)
{
uint8_t x_87; 
x_87 = !lean_is_exclusive(x_86);
if (x_87 == 0)
{
lean_object* x_88; lean_object* x_89; 
x_88 = lean_ctor_get(x_86, 0);
x_89 = lean_int_mul(x_85, x_88);
lean_dec(x_88);
lean_dec(x_85);
lean_ctor_set(x_86, 0, x_89);
return x_86;
}
else
{
lean_object* x_90; lean_object* x_91; lean_object* x_92; 
x_90 = lean_ctor_get(x_86, 0);
lean_inc(x_90);
lean_dec(x_86);
x_91 = lean_int_mul(x_85, x_90);
lean_dec(x_90);
lean_dec(x_85);
x_92 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_92, 0, x_91);
return x_92;
}
}
else
{
lean_object* x_93; 
lean_dec(x_86);
lean_dec(x_85);
x_93 = lean_box(5);
return x_93;
}
}
else
{
lean_object* x_94; 
lean_dec(x_84);
lean_dec(x_83);
lean_dec(x_1);
x_94 = lean_box(5);
return x_94;
}
}
case 5:
{
lean_object* x_95; lean_object* x_96; lean_object* x_97; lean_object* x_98; uint8_t x_99; lean_object* x_100; 
x_95 = lean_ctor_get(x_2, 0);
lean_inc(x_95);
x_96 = lean_ctor_get(x_2, 1);
lean_inc(x_96);
lean_dec_ref(x_2);
lean_inc(x_1);
x_97 = lp_NeveFormal_eval(x_1, x_95);
x_98 = lp_NeveFormal_eval(x_1, x_96);
x_99 = lp_NeveFormal_Neve_instBEqValue_beq(x_97, x_98);
x_100 = lean_alloc_ctor(2, 0, 1);
lean_ctor_set_uint8(x_100, 0, x_99);
return x_100;
}
case 11:
{
lean_object* x_101; lean_object* x_102; lean_object* x_103; 
x_101 = lean_ctor_get(x_2, 0);
lean_inc(x_101);
x_102 = lean_ctor_get(x_2, 1);
lean_inc(x_102);
lean_dec_ref(x_2);
lean_inc(x_1);
x_103 = lp_NeveFormal_eval(x_1, x_101);
if (lean_obj_tag(x_103) == 2)
{
uint8_t x_104; 
x_104 = lean_ctor_get_uint8(x_103, 0);
if (x_104 == 0)
{
lean_dec(x_102);
lean_dec(x_1);
return x_103;
}
else
{
lean_dec_ref(x_103);
x_2 = x_102;
goto _start;
}
}
else
{
lean_object* x_106; 
lean_dec(x_103);
lean_dec(x_102);
lean_dec(x_1);
x_106 = lean_box(5);
return x_106;
}
}
case 12:
{
lean_object* x_107; lean_object* x_108; lean_object* x_109; 
x_107 = lean_ctor_get(x_2, 0);
lean_inc(x_107);
x_108 = lean_ctor_get(x_2, 1);
lean_inc(x_108);
lean_dec_ref(x_2);
lean_inc(x_1);
x_109 = lp_NeveFormal_eval(x_1, x_107);
if (lean_obj_tag(x_109) == 2)
{
uint8_t x_110; 
x_110 = lean_ctor_get_uint8(x_109, 0);
if (x_110 == 0)
{
lean_dec_ref(x_109);
x_2 = x_108;
goto _start;
}
else
{
lean_dec(x_108);
lean_dec(x_1);
return x_109;
}
}
else
{
lean_object* x_112; 
lean_dec(x_109);
lean_dec(x_108);
lean_dec(x_1);
x_112 = lean_box(5);
return x_112;
}
}
default: 
{
lean_object* x_113; 
lean_dec_ref(x_2);
lean_dec(x_1);
x_113 = lean_box(5);
return x_113;
}
}
}
case 11:
{
lean_object* x_114; lean_object* x_115; lean_object* x_116; lean_object* x_117; 
x_114 = lean_ctor_get(x_2, 0);
lean_inc(x_114);
x_115 = lean_ctor_get(x_2, 1);
lean_inc(x_115);
lean_dec_ref(x_2);
lean_inc(x_1);
x_116 = lp_NeveFormal_eval(x_1, x_114);
x_117 = lp_NeveFormal_findArm(x_116, x_115);
lean_dec(x_115);
lean_dec(x_116);
if (lean_obj_tag(x_117) == 0)
{
lean_object* x_118; 
lean_dec(x_1);
x_118 = lean_box(5);
return x_118;
}
else
{
lean_object* x_119; lean_object* x_120; 
x_119 = lean_ctor_get(x_117, 0);
lean_inc(x_119);
lean_dec_ref(x_117);
x_120 = lean_ctor_get(x_119, 1);
lean_inc(x_120);
lean_dec(x_119);
x_2 = x_120;
goto _start;
}
}
default: 
{
lean_object* x_122; 
lean_dec(x_2);
lean_dec(x_1);
x_122 = lean_box(5);
return x_122;
}
}
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_List_lookup___at___00eval_spec__0(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_4; 
x_4 = lp_NeveFormal_List_lookup___at___00eval_spec__0___redArg(x_2, x_3);
return x_4;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_List_lookup___at___00eval_spec__0___boxed(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_4; 
x_4 = lp_NeveFormal_List_lookup___at___00eval_spec__0(x_1, x_2, x_3);
lean_dec(x_3);
lean_dec_ref(x_2);
return x_4;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_evalClosed(lean_object* x_1) {
_start:
{
lean_object* x_2; lean_object* x_3; 
x_2 = lean_box(0);
x_3 = lp_NeveFormal_eval(x_2, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_fmt___closed__0(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(0u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_fmt(lean_object* x_1) {
_start:
{
switch (lean_obj_tag(x_1)) {
case 0:
{
lean_object* x_2; lean_object* x_3; uint8_t x_4; 
x_2 = lean_ctor_get(x_1, 0);
x_3 = lean_obj_once(&lp_NeveFormal_fmt___closed__0, &lp_NeveFormal_fmt___closed__0_once, _init_lp_NeveFormal_fmt___closed__0);
x_4 = lean_int_dec_lt(x_2, x_3);
if (x_4 == 0)
{
lean_object* x_5; lean_object* x_6; 
x_5 = lean_nat_abs(x_2);
x_6 = l_Nat_reprFast(x_5);
return x_6;
}
else
{
lean_object* x_7; lean_object* x_8; lean_object* x_9; lean_object* x_10; lean_object* x_11; lean_object* x_12; lean_object* x_13; 
x_7 = lean_nat_abs(x_2);
x_8 = lean_unsigned_to_nat(1u);
x_9 = lean_nat_sub(x_7, x_8);
lean_dec(x_7);
x_10 = ((lean_object*)(lp_NeveFormal_fmt___closed__1));
x_11 = lean_nat_add(x_9, x_8);
lean_dec(x_9);
x_12 = l_Nat_reprFast(x_11);
x_13 = lean_string_append(x_10, x_12);
lean_dec_ref(x_12);
return x_13;
}
}
case 2:
{
uint8_t x_14; 
x_14 = lean_ctor_get_uint8(x_1, 0);
if (x_14 == 0)
{
lean_object* x_15; 
x_15 = ((lean_object*)(lp_NeveFormal_fmt___closed__2));
return x_15;
}
else
{
lean_object* x_16; 
x_16 = ((lean_object*)(lp_NeveFormal_fmt___closed__3));
return x_16;
}
}
case 5:
{
lean_object* x_17; 
x_17 = ((lean_object*)(lp_NeveFormal_fmt___closed__4));
return x_17;
}
case 4:
{
lean_object* x_18; lean_object* x_19; lean_object* x_20; lean_object* x_21; 
x_18 = lean_ctor_get(x_1, 0);
x_19 = ((lean_object*)(lp_NeveFormal_fmt___closed__5));
x_20 = lean_string_append(x_19, x_18);
x_21 = lean_string_append(x_20, x_19);
return x_21;
}
default: 
{
lean_object* x_22; 
x_22 = ((lean_object*)(lp_NeveFormal_fmt___closed__6));
return x_22;
}
}
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_fmt___boxed(lean_object* x_1) {
_start:
{
lean_object* x_2; 
x_2 = lp_NeveFormal_fmt(x_1);
lean_dec(x_1);
return x_2;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_IO_print___at___00IO_println___at___00runTest_spec__0_spec__0(lean_object* x_1) {
_start:
{
lean_object* x_3; lean_object* x_4; lean_object* x_5; 
x_3 = lean_get_stdout();
x_4 = lean_ctor_get(x_3, 4);
lean_inc_ref(x_4);
lean_dec_ref(x_3);
x_5 = lean_apply_2(x_4, x_1, lean_box(0));
return x_5;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_IO_print___at___00IO_println___at___00runTest_spec__0_spec__0___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lp_NeveFormal_IO_print___at___00IO_println___at___00runTest_spec__0_spec__0(x_1);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_IO_println___at___00runTest_spec__0(lean_object* x_1) {
_start:
{
uint32_t x_3; lean_object* x_4; lean_object* x_5; 
x_3 = 10;
x_4 = lean_string_push(x_1, x_3);
x_5 = lp_NeveFormal_IO_print___at___00IO_println___at___00runTest_spec__0_spec__0(x_4);
return x_5;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_IO_println___at___00runTest_spec__0___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lp_NeveFormal_IO_println___at___00runTest_spec__0(x_1);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_runTest(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_5; lean_object* x_6; uint8_t x_7; 
x_5 = lp_NeveFormal_evalClosed(x_2);
x_6 = lp_NeveFormal_fmt(x_5);
lean_dec(x_5);
x_7 = lean_string_dec_eq(x_6, x_3);
if (x_7 == 0)
{
lean_object* x_8; lean_object* x_9; lean_object* x_10; lean_object* x_11; lean_object* x_12; lean_object* x_13; lean_object* x_14; lean_object* x_15; lean_object* x_16; 
x_8 = ((lean_object*)(lp_NeveFormal_runTest___closed__0));
x_9 = lean_string_append(x_8, x_1);
x_10 = ((lean_object*)(lp_NeveFormal_runTest___closed__1));
x_11 = lean_string_append(x_9, x_10);
x_12 = lean_string_append(x_11, x_6);
lean_dec_ref(x_6);
x_13 = ((lean_object*)(lp_NeveFormal_runTest___closed__2));
x_14 = lean_string_append(x_12, x_13);
x_15 = lean_string_append(x_14, x_3);
x_16 = lp_NeveFormal_IO_println___at___00runTest_spec__0(x_15);
return x_16;
}
else
{
lean_object* x_17; lean_object* x_18; lean_object* x_19; lean_object* x_20; lean_object* x_21; lean_object* x_22; 
x_17 = ((lean_object*)(lp_NeveFormal_runTest___closed__3));
x_18 = lean_string_append(x_17, x_1);
x_19 = ((lean_object*)(lp_NeveFormal_runTest___closed__4));
x_20 = lean_string_append(x_18, x_19);
x_21 = lean_string_append(x_20, x_6);
lean_dec_ref(x_6);
x_22 = lp_NeveFormal_IO_println___at___00runTest_spec__0(x_21);
return x_22;
}
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_runTest___boxed(lean_object* x_1, lean_object* x_2, lean_object* x_3, lean_object* x_4) {
_start:
{
lean_object* x_5; 
x_5 = lp_NeveFormal_runTest(x_1, x_2, x_3);
lean_dec_ref(x_3);
lean_dec_ref(x_1);
return x_5;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__1(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(1u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__2(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__1, &lp_NeveFormal_main___closed__1_once, _init_lp_NeveFormal_main___closed__1);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__3(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(2u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__4(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__3, &lp_NeveFormal_main___closed__3_once, _init_lp_NeveFormal_main___closed__3);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__5(void) {
_start:
{
lean_object* x_1; lean_object* x_2; uint8_t x_3; lean_object* x_4; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__4, &lp_NeveFormal_main___closed__4_once, _init_lp_NeveFormal_main___closed__4);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__2, &lp_NeveFormal_main___closed__2_once, _init_lp_NeveFormal_main___closed__2);
x_3 = 0;
x_4 = lean_alloc_ctor(10, 2, 1);
lean_ctor_set(x_4, 0, x_2);
lean_ctor_set(x_4, 1, x_1);
lean_ctor_set_uint8(x_4, sizeof(void*)*2, x_3);
return x_4;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__8(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(3u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__9(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__8, &lp_NeveFormal_main___closed__8_once, _init_lp_NeveFormal_main___closed__8);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__10(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(4u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__11(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__10, &lp_NeveFormal_main___closed__10_once, _init_lp_NeveFormal_main___closed__10);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__12(void) {
_start:
{
lean_object* x_1; lean_object* x_2; uint8_t x_3; lean_object* x_4; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__11, &lp_NeveFormal_main___closed__11_once, _init_lp_NeveFormal_main___closed__11);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__9, &lp_NeveFormal_main___closed__9_once, _init_lp_NeveFormal_main___closed__9);
x_3 = 0;
x_4 = lean_alloc_ctor(10, 2, 1);
lean_ctor_set(x_4, 0, x_2);
lean_ctor_set(x_4, 1, x_1);
lean_ctor_set_uint8(x_4, sizeof(void*)*2, x_3);
return x_4;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__13(void) {
_start:
{
lean_object* x_1; lean_object* x_2; uint8_t x_3; lean_object* x_4; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__4, &lp_NeveFormal_main___closed__4_once, _init_lp_NeveFormal_main___closed__4);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__12, &lp_NeveFormal_main___closed__12_once, _init_lp_NeveFormal_main___closed__12);
x_3 = 2;
x_4 = lean_alloc_ctor(10, 2, 1);
lean_ctor_set(x_4, 0, x_2);
lean_ctor_set(x_4, 1, x_1);
lean_ctor_set_uint8(x_4, sizeof(void*)*2, x_3);
return x_4;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__16(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(10u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__17(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__16, &lp_NeveFormal_main___closed__16_once, _init_lp_NeveFormal_main___closed__16);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__18(void) {
_start:
{
lean_object* x_1; lean_object* x_2; uint8_t x_3; lean_object* x_4; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__9, &lp_NeveFormal_main___closed__9_once, _init_lp_NeveFormal_main___closed__9);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__17, &lp_NeveFormal_main___closed__17_once, _init_lp_NeveFormal_main___closed__17);
x_3 = 1;
x_4 = lean_alloc_ctor(10, 2, 1);
lean_ctor_set(x_4, 0, x_2);
lean_ctor_set(x_4, 1, x_1);
lean_ctor_set_uint8(x_4, sizeof(void*)*2, x_3);
return x_4;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__22(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(20u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__23(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__22, &lp_NeveFormal_main___closed__22_once, _init_lp_NeveFormal_main___closed__22);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__24(void) {
_start:
{
lean_object* x_1; lean_object* x_2; uint8_t x_3; lean_object* x_4; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__23, &lp_NeveFormal_main___closed__23_once, _init_lp_NeveFormal_main___closed__23);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__17, &lp_NeveFormal_main___closed__17_once, _init_lp_NeveFormal_main___closed__17);
x_3 = 0;
x_4 = lean_alloc_ctor(10, 2, 1);
lean_ctor_set(x_4, 0, x_2);
lean_ctor_set(x_4, 1, x_1);
lean_ctor_set_uint8(x_4, sizeof(void*)*2, x_3);
return x_4;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__26(void) {
_start:
{
lean_object* x_1; lean_object* x_2; uint8_t x_3; lean_object* x_4; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__4, &lp_NeveFormal_main___closed__4_once, _init_lp_NeveFormal_main___closed__4);
x_2 = ((lean_object*)(lp_NeveFormal_main___closed__25));
x_3 = 2;
x_4 = lean_alloc_ctor(10, 2, 1);
lean_ctor_set(x_4, 0, x_2);
lean_ctor_set(x_4, 1, x_1);
lean_ctor_set_uint8(x_4, sizeof(void*)*2, x_3);
return x_4;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__27(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; lean_object* x_4; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__26, &lp_NeveFormal_main___closed__26_once, _init_lp_NeveFormal_main___closed__26);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__24, &lp_NeveFormal_main___closed__24_once, _init_lp_NeveFormal_main___closed__24);
x_3 = ((lean_object*)(lp_NeveFormal_main___closed__21));
x_4 = lean_alloc_ctor(9, 3, 0);
lean_ctor_set(x_4, 0, x_3);
lean_ctor_set(x_4, 1, x_2);
lean_ctor_set(x_4, 2, x_1);
return x_4;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__30(void) {
_start:
{
lean_object* x_1; lean_object* x_2; uint8_t x_3; lean_object* x_4; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__2, &lp_NeveFormal_main___closed__2_once, _init_lp_NeveFormal_main___closed__2);
x_2 = ((lean_object*)(lp_NeveFormal_main___closed__25));
x_3 = 0;
x_4 = lean_alloc_ctor(10, 2, 1);
lean_ctor_set(x_4, 0, x_2);
lean_ctor_set(x_4, 1, x_1);
lean_ctor_set_uint8(x_4, sizeof(void*)*2, x_3);
return x_4;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__31(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__30, &lp_NeveFormal_main___closed__30_once, _init_lp_NeveFormal_main___closed__30);
x_2 = ((lean_object*)(lp_NeveFormal_main___closed__21));
x_3 = lean_alloc_ctor(8, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__32(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(41u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__33(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__32, &lp_NeveFormal_main___closed__32_once, _init_lp_NeveFormal_main___closed__32);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__34(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__33, &lp_NeveFormal_main___closed__33_once, _init_lp_NeveFormal_main___closed__33);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__31, &lp_NeveFormal_main___closed__31_once, _init_lp_NeveFormal_main___closed__31);
x_3 = lean_alloc_ctor(7, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__37(void) {
_start:
{
lean_object* x_1; uint8_t x_2; lean_object* x_3; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__2, &lp_NeveFormal_main___closed__2_once, _init_lp_NeveFormal_main___closed__2);
x_2 = 5;
x_3 = lean_alloc_ctor(10, 2, 1);
lean_ctor_set(x_3, 0, x_1);
lean_ctor_set(x_3, 1, x_1);
lean_ctor_set_uint8(x_3, sizeof(void*)*2, x_2);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__39(void) {
_start:
{
lean_object* x_1; lean_object* x_2; uint8_t x_3; lean_object* x_4; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__4, &lp_NeveFormal_main___closed__4_once, _init_lp_NeveFormal_main___closed__4);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__2, &lp_NeveFormal_main___closed__2_once, _init_lp_NeveFormal_main___closed__2);
x_3 = 5;
x_4 = lean_alloc_ctor(10, 2, 1);
lean_ctor_set(x_4, 0, x_2);
lean_ctor_set(x_4, 1, x_1);
lean_ctor_set_uint8(x_4, sizeof(void*)*2, x_3);
return x_4;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__47(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(42u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__48(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__47, &lp_NeveFormal_main___closed__47_once, _init_lp_NeveFormal_main___closed__47);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__49(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(100u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__50(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__49, &lp_NeveFormal_main___closed__49_once, _init_lp_NeveFormal_main___closed__49);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__51(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__50, &lp_NeveFormal_main___closed__50_once, _init_lp_NeveFormal_main___closed__50);
x_2 = lean_box(0);
x_3 = lean_alloc_ctor(0, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__52(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_box(0);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__51, &lp_NeveFormal_main___closed__51_once, _init_lp_NeveFormal_main___closed__51);
x_3 = lean_alloc_ctor(1, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__53(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__52, &lp_NeveFormal_main___closed__52_once, _init_lp_NeveFormal_main___closed__52);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__48, &lp_NeveFormal_main___closed__48_once, _init_lp_NeveFormal_main___closed__48);
x_3 = lean_alloc_ctor(11, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__57(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__2, &lp_NeveFormal_main___closed__2_once, _init_lp_NeveFormal_main___closed__2);
x_2 = ((lean_object*)(lp_NeveFormal_main___closed__56));
x_3 = lean_alloc_ctor(0, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__59(void) {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_obj_once(&lp_NeveFormal_fmt___closed__0, &lp_NeveFormal_fmt___closed__0_once, _init_lp_NeveFormal_fmt___closed__0);
x_2 = lean_alloc_ctor(0, 1, 0);
lean_ctor_set(x_2, 0, x_1);
return x_2;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__60(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__59, &lp_NeveFormal_main___closed__59_once, _init_lp_NeveFormal_main___closed__59);
x_2 = ((lean_object*)(lp_NeveFormal_main___closed__58));
x_3 = lean_alloc_ctor(0, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__61(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_box(0);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__60, &lp_NeveFormal_main___closed__60_once, _init_lp_NeveFormal_main___closed__60);
x_3 = lean_alloc_ctor(1, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__62(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__61, &lp_NeveFormal_main___closed__61_once, _init_lp_NeveFormal_main___closed__61);
x_2 = lean_obj_once(&lp_NeveFormal_main___closed__57, &lp_NeveFormal_main___closed__57_once, _init_lp_NeveFormal_main___closed__57);
x_3 = lean_alloc_ctor(1, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
static lean_object* _init_lp_NeveFormal_main___closed__63(void) {
_start:
{
lean_object* x_1; lean_object* x_2; lean_object* x_3; 
x_1 = lean_obj_once(&lp_NeveFormal_main___closed__62, &lp_NeveFormal_main___closed__62_once, _init_lp_NeveFormal_main___closed__62);
x_2 = ((lean_object*)(lp_NeveFormal_main___closed__41));
x_3 = lean_alloc_ctor(11, 2, 0);
lean_ctor_set(x_3, 0, x_2);
lean_ctor_set(x_3, 1, x_1);
return x_3;
}
}
LEAN_EXPORT lean_object* _lean_main() {
_start:
{
lean_object* x_2; lean_object* x_3; lean_object* x_4; lean_object* x_5; 
x_2 = ((lean_object*)(lp_NeveFormal_main___closed__0));
x_3 = lean_obj_once(&lp_NeveFormal_main___closed__5, &lp_NeveFormal_main___closed__5_once, _init_lp_NeveFormal_main___closed__5);
x_4 = ((lean_object*)(lp_NeveFormal_main___closed__6));
x_5 = lp_NeveFormal_runTest(x_2, x_3, x_4);
if (lean_obj_tag(x_5) == 0)
{
lean_object* x_6; lean_object* x_7; lean_object* x_8; lean_object* x_9; 
lean_dec_ref(x_5);
x_6 = ((lean_object*)(lp_NeveFormal_main___closed__7));
x_7 = lean_obj_once(&lp_NeveFormal_main___closed__13, &lp_NeveFormal_main___closed__13_once, _init_lp_NeveFormal_main___closed__13);
x_8 = ((lean_object*)(lp_NeveFormal_main___closed__14));
x_9 = lp_NeveFormal_runTest(x_6, x_7, x_8);
if (lean_obj_tag(x_9) == 0)
{
lean_object* x_10; lean_object* x_11; lean_object* x_12; lean_object* x_13; 
lean_dec_ref(x_9);
x_10 = ((lean_object*)(lp_NeveFormal_main___closed__15));
x_11 = lean_obj_once(&lp_NeveFormal_main___closed__18, &lp_NeveFormal_main___closed__18_once, _init_lp_NeveFormal_main___closed__18);
x_12 = ((lean_object*)(lp_NeveFormal_main___closed__19));
x_13 = lp_NeveFormal_runTest(x_10, x_11, x_12);
if (lean_obj_tag(x_13) == 0)
{
lean_object* x_14; lean_object* x_15; lean_object* x_16; lean_object* x_17; 
lean_dec_ref(x_13);
x_14 = ((lean_object*)(lp_NeveFormal_main___closed__20));
x_15 = lean_obj_once(&lp_NeveFormal_main___closed__27, &lp_NeveFormal_main___closed__27_once, _init_lp_NeveFormal_main___closed__27);
x_16 = ((lean_object*)(lp_NeveFormal_main___closed__28));
x_17 = lp_NeveFormal_runTest(x_14, x_15, x_16);
if (lean_obj_tag(x_17) == 0)
{
lean_object* x_18; lean_object* x_19; lean_object* x_20; lean_object* x_21; 
lean_dec_ref(x_17);
x_18 = ((lean_object*)(lp_NeveFormal_main___closed__29));
x_19 = lean_obj_once(&lp_NeveFormal_main___closed__34, &lp_NeveFormal_main___closed__34_once, _init_lp_NeveFormal_main___closed__34);
x_20 = ((lean_object*)(lp_NeveFormal_main___closed__35));
x_21 = lp_NeveFormal_runTest(x_18, x_19, x_20);
if (lean_obj_tag(x_21) == 0)
{
lean_object* x_22; lean_object* x_23; lean_object* x_24; lean_object* x_25; 
lean_dec_ref(x_21);
x_22 = ((lean_object*)(lp_NeveFormal_main___closed__36));
x_23 = lean_obj_once(&lp_NeveFormal_main___closed__37, &lp_NeveFormal_main___closed__37_once, _init_lp_NeveFormal_main___closed__37);
x_24 = ((lean_object*)(lp_NeveFormal_fmt___closed__3));
x_25 = lp_NeveFormal_runTest(x_22, x_23, x_24);
if (lean_obj_tag(x_25) == 0)
{
lean_object* x_26; lean_object* x_27; lean_object* x_28; lean_object* x_29; 
lean_dec_ref(x_25);
x_26 = ((lean_object*)(lp_NeveFormal_main___closed__38));
x_27 = lean_obj_once(&lp_NeveFormal_main___closed__39, &lp_NeveFormal_main___closed__39_once, _init_lp_NeveFormal_main___closed__39);
x_28 = ((lean_object*)(lp_NeveFormal_fmt___closed__2));
x_29 = lp_NeveFormal_runTest(x_26, x_27, x_28);
if (lean_obj_tag(x_29) == 0)
{
lean_object* x_30; lean_object* x_31; lean_object* x_32; 
lean_dec_ref(x_29);
x_30 = ((lean_object*)(lp_NeveFormal_main___closed__40));
x_31 = ((lean_object*)(lp_NeveFormal_main___closed__43));
x_32 = lp_NeveFormal_runTest(x_30, x_31, x_28);
if (lean_obj_tag(x_32) == 0)
{
lean_object* x_33; lean_object* x_34; lean_object* x_35; 
lean_dec_ref(x_32);
x_33 = ((lean_object*)(lp_NeveFormal_main___closed__44));
x_34 = ((lean_object*)(lp_NeveFormal_main___closed__45));
x_35 = lp_NeveFormal_runTest(x_33, x_34, x_24);
if (lean_obj_tag(x_35) == 0)
{
lean_object* x_36; lean_object* x_37; lean_object* x_38; lean_object* x_39; 
lean_dec_ref(x_35);
x_36 = ((lean_object*)(lp_NeveFormal_main___closed__46));
x_37 = lean_obj_once(&lp_NeveFormal_main___closed__53, &lp_NeveFormal_main___closed__53_once, _init_lp_NeveFormal_main___closed__53);
x_38 = ((lean_object*)(lp_NeveFormal_main___closed__54));
x_39 = lp_NeveFormal_runTest(x_36, x_37, x_38);
if (lean_obj_tag(x_39) == 0)
{
lean_object* x_40; lean_object* x_41; lean_object* x_42; lean_object* x_43; 
lean_dec_ref(x_39);
x_40 = ((lean_object*)(lp_NeveFormal_main___closed__55));
x_41 = lean_obj_once(&lp_NeveFormal_main___closed__63, &lp_NeveFormal_main___closed__63_once, _init_lp_NeveFormal_main___closed__63);
x_42 = ((lean_object*)(lp_NeveFormal_main___closed__64));
x_43 = lp_NeveFormal_runTest(x_40, x_41, x_42);
if (lean_obj_tag(x_43) == 0)
{
lean_object* x_44; lean_object* x_45; 
lean_dec_ref(x_43);
x_44 = ((lean_object*)(lp_NeveFormal_main___closed__65));
x_45 = lp_NeveFormal_IO_println___at___00runTest_spec__0(x_44);
return x_45;
}
else
{
return x_43;
}
}
else
{
return x_39;
}
}
else
{
return x_35;
}
}
else
{
return x_32;
}
}
else
{
return x_29;
}
}
else
{
return x_25;
}
}
else
{
return x_21;
}
}
else
{
return x_17;
}
}
else
{
return x_13;
}
}
else
{
return x_9;
}
}
else
{
return x_5;
}
}
}
LEAN_EXPORT lean_object* lp_NeveFormal_main___boxed(lean_object* x_1) {
_start:
{
lean_object* x_2; 
x_2 = _lean_main();
return x_2;
}
}
lean_object* initialize_Init(uint8_t builtin);
lean_object* initialize_NeveFormal_Neve_Spec_Syntax(uint8_t builtin);
static bool _G_initialized = false;
LEAN_EXPORT lean_object* initialize_NeveFormal_Neve_Tests_Eval(uint8_t builtin) {
lean_object * res;
if (_G_initialized) return lean_io_result_mk_ok(lean_box(0));
_G_initialized = true;
res = initialize_Init(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
res = initialize_NeveFormal_Neve_Spec_Syntax(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
return lean_io_result_mk_ok(lean_box(0));
}
char ** lean_setup_args(int argc, char ** argv);
void lean_initialize_runtime_module();

  #if defined(WIN32) || defined(_WIN32)
  #include <windows.h>
  #endif

  int main(int argc, char ** argv) {
  #if defined(WIN32) || defined(_WIN32)
  SetErrorMode(SEM_FAILCRITICALERRORS);
  SetConsoleOutputCP(CP_UTF8);
  #endif
  lean_object* in; lean_object* res;
argv = lean_setup_args(argc, argv);
lean_initialize_runtime_module();
lean_set_panic_messages(false);
res = initialize_NeveFormal_Neve_Tests_Eval(1 /* builtin */);
lean_set_panic_messages(true);
lean_io_mark_end_initialization();
if (lean_io_result_is_ok(res)) {
lean_dec_ref(res);
lean_init_task_manager();
res = _lean_main();
}
lean_finalize_task_manager();
if (lean_io_result_is_ok(res)) {
  int ret = 0;
  lean_dec_ref(res);
  return ret;
} else {
  lean_io_result_show_error(res);
  lean_dec_ref(res);
  return 1;
}
}
#ifdef __cplusplus
}
#endif
