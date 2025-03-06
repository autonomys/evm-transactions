// SPDX-License-Identifier: MIT
pragma solidity 0.8.19;

/**
 * This contract is about load testing which means we need to see if high
 * computing function execution can be handled by the network or not
 */
contract Load {
    // ===========Storage==========================
    mapping(address => uint256[]) public arr1;

    event SetArrayValue(
        address wallet,
        uint256 index,
        uint256 value,
        uint256 count
    );

    event SetArrayComplete(address wallet, uint256 count, uint256 timestamp);

    // ===========Functions=========================

    /// @dev Calculate factorial of a number
    function factorial(uint256 num) external pure returns (uint256) {
        uint256 fact = 1;
        for (uint256 i = 1; i <= num; ++i) {
            fact = fact * i;
        }

        return fact;
    }

    /// @dev Set values in an array
    function setArray(uint256 count) external {
        uint256[] memory arr = new uint256[](count);
        arr[0] = 1;
        arr[1] = 2;
        arr[2] = 3;

        for (uint256 i = 0; i < count; ++i) {
            uint256 value = i * i * i * i;
            arr[i] = value;
            emit SetArrayValue(msg.sender, i, value, count);
        }

        arr1[msg.sender] = arr;
        emit SetArrayComplete(msg.sender, count, block.timestamp);
    }
}
