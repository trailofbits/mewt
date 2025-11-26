// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract HelloWorld {
    function greet(uint a, uint b) public pure returns (uint) {
        if (a > b) {
            return a - b;
        }
        return a + b;
    }
}




