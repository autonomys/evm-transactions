// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Test, console2} from "forge-std/Test.sol";
import {Load} from "../src/Load.sol";

contract LoadTest is Test {
    Load public load;

    function setUp() public {
        load = new Load();
    }

    function test_Factorial() public {
        uint256 fact = load.factorial(10);
        assertEq(fact, 3628800);
    }

    /// fuzz test for factorial
    function testFuzz_Factorial(uint256 num) public view {
        vm.assume(num > 0 && num < 58);
        uint256 fact = load.factorial(num);
    }
}
