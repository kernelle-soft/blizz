// Test file with ignore directives

// violet-ignore max-params
function allowedManyParams(a, b, c, d, e, f) {
    return a + b + c + d + e + f;
}

function stillTooManyParams(a, b, c, d, e) {
    return a + b + c + d + e;
}

// violet-ignore function-depth
function allowedNested() {
    if (true) {
        if (true) {
            if (true) {
                if (true) {
                    return "deeply nested but ignored";
                }
            }
        }
    }
} 