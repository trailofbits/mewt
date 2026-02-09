// TypeScript generics that should NOT be mutated by COS
const eventEmitter = module.get<EventEmitter2>(EventEmitter2);
const result = foo<string, number>(arg1, arg2);

function generic<T, U>(a: T, b: U): T {
    return a;
}

class Container<T> {
    private value: T;

    constructor(val: T) {
        this.value = val;
    }

    get<U = T>(): U {
        return this.value as unknown as U;
    }
}

// Real comparisons that SHOULD be mutated by COS
if (a < b && c > d) {
    return true;
}

const max = x >= y ? x : y;
const isEqual = foo == bar;
const isNotEqual = baz !== qux;

for (let i = 0; i < 10; i++) {
    if (i <= 5) {
        console.log("small");
    }
}
