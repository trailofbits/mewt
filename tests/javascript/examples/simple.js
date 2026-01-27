function add(a, b) {
    return a + b;
}

function isPositive(x) {
    if (x > 0) {
        return true;
    }
    return false;
}

function calculate(a, b, op) {
    if (op === 'add') {
        return a + b;
    } else if (op === 'sub') {
        return a - b;
    } else if (op === 'mul') {
        return a * b;
    }
    return 0;
}
