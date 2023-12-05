// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Script, console2} from "forge-std/Script.sol";
import {Load} from "../src/Load.sol";

/* 
    $ source .env
    $ forge script script/Load.s.sol:LoadContractScript --rpc-url $SUBSPACE_EVM_RPC_URL --private-key $DEPLOYER_PRIVATE_KEY --broadcast --verify
*/

contract LoadContractScript is Script {
    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        Load load = new Load();
        console2.log("Load SC deployed at", address(load));

        vm.stopBroadcast();
    }
}
