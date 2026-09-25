use super::CompletionSpec;

pub(super) fn specs() -> Vec<CompletionSpec> {
    vec![
        (
            "math.toInt",
            "Convert to integer",
            "math.toInt(${1:x})",
            "Int",
        ),
        (
            "math.toFloat",
            "Convert to float",
            "math.toFloat(${1:x})",
            "Float",
        ),
        ("math.isNan", "Check for NaN", "math.isNan(${1:x})", "Bool"),
        (
            "math.isInf",
            "Check for infinity",
            "math.isInf(${1:x})",
            "Bool",
        ),
        ("math.floor", "Floor of float", "math.floor(${1:x})", "Int"),
        ("math.ceil", "Ceiling of float", "math.ceil(${1:x})", "Int"),
        ("math.round", "Round float", "math.round(${1:x})", "Int"),
        ("math.sqrt", "Square root", "math.sqrt(${1:x})", "Float"),
        ("math.log", "Natural logarithm", "math.log(${1:x})", "Float"),
        (
            "math.log10",
            "Base-10 logarithm",
            "math.log10(${1:x})",
            "Float",
        ),
        ("math.exp", "Exponential", "math.exp(${1:x})", "Float"),
        ("math.sin", "Sine", "math.sin(${1:x})", "Float"),
        ("math.cos", "Cosine", "math.cos(${1:x})", "Float"),
        ("math.tan", "Tangent", "math.tan(${1:x})", "Float"),
        ("math.pi", "Pi constant", "math.pi", "Float"),
        ("math.e", "Euler's number", "math.e", "Float"),
        ("math.inf", "Infinity constant", "math.inf", "Float"),
        ("math.nan", "NaN constant", "math.nan", "Float"),
    ]
}
