# Daemon Interchain Environment

This environment allows to interact with actual COSMOS SDK Nodes. Let's see how that work in details: 


## Environment creation

### For scripting

When scripting with `cw-orch-interchain`, developers don't have to create chain `Daemon` objects on their own. You can simply pass chain data to the interchain constructor, and it will create the daemons for you. Like so:

```rust,ignore
use cw_orch::prelude::*;
use cw_orch::tokio::runtime::Runtime;
use cw_orch::prelude::networks::{LOCAL_JUNO, LOCAL_OSMO};
use cw_orch_interchain::interchain::{ChannelCreationValidator,DaemonInterchainEnv};
# fn main(){
    let rt = Runtime::new()?;
    let mut interchain = DaemonInterchainEnv::new(vec![
        (LOCAL_JUNO, None),
        (LOCAL_OSMO, None)
    ], &ChannelCreationValidator)?;
# }
```

You can then access individual `Daemon` objects like so:

```rust,ignore
use cw_orch::prelude::*;
use cw_orch_interchain::interchain::InterchainEnv;
# fn main(){
    let local_juno: Daemon = interchain.chain("testing")?;
    let local_osmo: Daemon = interchain.chain("localosmosis")?;
#  }
```

where the argument of the `chain` method is the chain id of the chain you are interacting with. Note that this environment can't work with chains that have the same `chain_id`. 

> **NOTE**: Here the `ChannelCreationValidator` struct is a helper that will simply wait for channel creation when it's called in the script. [More information on that channel creation later](#ibc-channel-creation).

You can also add daemons manually to the `interchain` object:

```rust,ignore
let local_migaloo = DaemonBuilder::default()
    .chain(LOCAL_MIGALOO)
    .build()?;
interchain.add_daemons(vec![local_migaloo]);
```

### For testing

In some cases (we highly recommand it), you might want to interact with local nodes and relayers to test IBC interactions. To do so, we allow users to leverage <a href="https://starship.cosmology.tech/" target="_blank">Starship</a>. Starship is developed by <a href="https://twitter.com/cosmology_tech" target="_blank">@cosmology_tech</a> and allows developers to spin up a fully simulated mini-cosmos ecosystem. It sets up Cosmos SDK Nodes as well as relayers between them allowing you to focus on your application and less on the testing environment.

For setup, please refer to <a href="https://starship.cosmology.tech/get-started/step-1" target="_blank">the official Quick Start</a>. When all that is done, the starship adapter that we provide will detect the deployment and create the right `cw-orchestrator` variables and structures for you to interact and test with.

```rust,ignore
use cw_orch_interchain::interchain::{Starship, ChannelCreator};

# fn main(){
    let rt = Runtime::new()?;
    let starship = Starship::new(None)?;
    let interchain = starship.interchain_env();

    let _local_juno: Daemon = interchain.chain("juno-1")?;
    let _local_osmo: Daemon = interchain.chain("osmosis-1")?;
# }
```

> **NOTE**: The second argument of the `Starship::new` function is the optional URL of the starship deployment. It defaults to `http://localhost:8081`, but you can customize it if it doesn't match your setup. All the starship data, daemons and relayer setup is loaded from that URL.

## General Usage

All interchain environments are centered around the `follow_packet` function. In the Daemon case (be it for testing or for scripting), this function is responsible for tracking the relayer interactions associated with the packet lifetime. The lifetime steps of this function are:

1. <span style="color:purple">⬤</span> On the `source chain`, identify the packet and the destination chain. If the destination chain id is not registered in the `interchain` environment, it will error. Please make sure all the chains your are trying to inspect are included in the environment. 
2. Then, it follows the time line of a packet. A packet can either timeout or be transmitted successfully. The function concurrently does the following steps. If one step returns successfully, the other step will be aborted (as a packet can only have one outcome).
    a. Successful cycle:
      1. <span style="color:red">⬤</span> On the `destination chain`, it looks for the receive transaction of that packet. The function logs the transaction hash as well as the acknowledgement when the receive transaction is found.
      2. <span style="color:purple">⬤</span> On the `source chain`, it looks for the acknowledgement transaction of that packet. The function logs when the acknowledgement is received and returns with the transactions involved in the packet broadcast, as well as information about the acknowledgement. 
    b. Timeout:
      1. <span style="color:purple">⬤</span> On the `source chain`, it looks for the timeout transaction for that packet. The function logs the transaction hash of the transaction and returns the transaction response corresponding to that transaction. 

If you have followed the usage closely, you see that this function doesn't error when the acknowledgement is an error, has a wrong format or if the packet timeouts. However, the function might error if either of the timeout/successful cycle takes too long. You can customize the wait time in the [cw-orchestrator environment variables](../../contracts/env-variable.md). 


The `wait_ibc` function is very similar except that instead of following a single packet, it follows all packets that are being sent within a transaction. This works in a very similar manner and will also not error as long as either a timeout or a successful cycle can be identified before `cw-orchestrator` query function timeouts. This function is recursive as it will also look for packets inside the receive/ack/timeout transactions and also follow their IBC cycle. You can think of this function as going down the rabbit-hole of IBC execution and only returning when all IBC interactions are complete. 

## Analysis Usage

The `follow_packet` and `wait_ibc` function were coded for scripting usage in mind. They allow to await and repeatedly query Cosmos SDK Nodes until the cycle is complete. However, it is also possible to inspect past transactions using those tools.
Using the `DaemonInterchainEnv::wait_ibc_from_txhash` function, one can inspect the history of packets linked to a transaction from a transaction hash only. This enables all kinds of analysis usage, here are some:

- Relayer activity
- Analysis of past transactions for fund recovery
- Whale account analysis
- ...

## IBC Channel creation

cw-orchestrator doesn't provide[^documentation_date] relayer capabilities. We only provide tools to analyze IBC activity based on packet relaying mechanism that only relayers can provide. However, when testing your implementation with Starship, you might want to automatically create channels on your test setup.

This is what the second argument of the `DaemonInterchainEnv::new` function is used for. You provide an object which will be responsible for creating an IBC channel between two ports. We provide 2 such structures, you can obviously create your own if your needs differ:

1. `cw_orch_interchain::interchain::ChannelCreationValidator`
    This is used when you want to have full control over the channel creation. When `interchain.create_channel` is called, the script will stop and prompt you to create a channel with external tools. Once the channel creation process is done on your side, you simply have to input the connection-id on which you created the channel to be able to resume execution. This solution is not ideal at all but allows you to script on actual nodes without having to separate your scripts into multiple parts or change the syntax you coded for your tests.

    To create the interchain environment with this `ChannelCreator`, [use the Validator syntax above](#for-scripting).

2. `cw_orch_interchain::interchain::Starship`

    This is used when testing your application with Starship. When `interchain.create_channel` is called, the script will simply send a command to the starship cluster to create an IBC channel between the chains that you specified. Obviously, the relayer has to be specified in the starship configuration for this function to return successfully. With this function, you don't have to worry about anything once your starship cluster is setup properly. The connection-id is returned automatically by the starship library and used throughout after that.

    To create the interchain environment with this `ChannelCreator`, [use the Starship syntax above](#for-testing).

[^documentation_date]: as of writing this documentation 09/28/2023
