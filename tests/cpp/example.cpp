#include <iostream>
#include <vector>
#include <string>

int add(int a, int b) {
    return a + b;
}

bool is_positive(int x) {
    return x > 0;
}

int factorial(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

int sum_vector(const std::vector<int>& vec) {
    int total = 0;
    for (const auto& val : vec) {
        total += val;
    }
    return total;
}

int count_down(int start) {
    int result = 0;
    while (start > 0) {
        result += start;
        start--;
    }
    return result;
}

void process(int x) {
    if (x < 0) {
        return;
    }
    if (x == 0 || x == 1) {
        std::cout << "small" << std::endl;
        return;
    }
    for (int i = 0; i < x; i++) {
        if (i % 2 == 0) {
            continue;
        }
        if (i > 10) {
            break;
        }
        std::cout << i << std::endl;
    }
}

bool check(bool a, bool b) {
    if (!a) {
        return false;
    }
    return a && b;
}

int bitwise(int a, int b) {
    int x = a & b;
    int y = a | b;
    int z = a ^ b;
    int s = a << 2;
    int r = b >> 1;
    return x + y + z + s + r;
}
