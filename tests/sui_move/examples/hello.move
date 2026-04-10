module hello::world {
    public struct Counter has key {
        id: sui::object::UID,
        count: u64,
    }

    public fun increment(counter: &mut Counter) {
        counter.count = counter.count + 1;
    }

    public fun get_count(counter: &Counter): u64 {
        counter.count
    }

    fun check_positive(value: u64): bool {
        if (value > 0) {
            true
        } else {
            false
        }
    }

    fun clamp(value: u64, min: u64, max: u64): u64 {
        if (value < min) {
            min
        } else if (value > max) {
            max
        } else {
            value
        }
    }

    fun sum_to(n: u64): u64 {
        let mut i = 0u64;
        let mut sum = 0u64;
        while (i < n) {
            sum = sum + i;
            i = i + 1;
        };
        sum
    }

    fun is_even(n: u64): bool {
        n % 2 == 0
    }

    fun safe_divide(a: u64, b: u64): u64 {
        if (b == 0) {
            abort 0
        };
        a / b
    }
}
