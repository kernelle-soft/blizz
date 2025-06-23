// Test file with various complexity issues

function tooManyParams(a, b, c, d, e, f) {
    return a + b + c + d + e + f;
}

function tooNested() {
    if (true) {
        if (true) {
            if (true) {
                if (true) {
                    return "deeply nested";
                }
            }
        }
    }
}

function tooComplex(x, y, z) {
    if (x > 0) {
        if (y > 0) {
            if (z > 0) {
                return 1;
            } else if (z < 0) {
                return -1;
            } else {
                return 0;
            }
        } else if (y < 0) {
            if (z > 0) {
                return 2;
            } else {
                return -2;
            }
        }
    } else if (x < 0) {
        return -3;
    } else {
        return 0;
    }
}

// This function should be fine
function goodFunction(a, b) {
    return a + b;
} 