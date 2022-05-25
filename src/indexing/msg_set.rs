use std::collections::HashSet;
use std::sync::{Arc, Mutex};

type MsgMap = HashSet<String>;

pub type MsgSet = Arc<Mutex<MsgSetInternal>>;

pub fn default_msg_set() -> MsgSet {
    Arc::new(Mutex::new(MsgSetInternal::new()))
}

#[derive(Clone, Debug)]
pub struct MsgSetInternal {
    pub registered_msgs: MsgMap,
    pub unregistered_msgs: MsgMap,
}

impl MsgSetInternal {
    pub fn new() -> Self {
        let mut registered_msgs = HashSet::new();
        init_known_unknown_messages(&mut registered_msgs);
        MsgSetInternal {
            registered_msgs,
            unregistered_msgs: HashSet::new(),
        }
    }

    pub fn validate(&mut self, msg: &str) -> bool {
        let mut result = true;
        if !self.registered_msgs.contains(msg) {
            result = self.unregistered_msgs.contains(msg);
            if !result {
                self.unregistered_msgs.insert(msg.to_string());
                result = true;
            }
        }
        result
    }
}

impl Default for MsgSetInternal {
    fn default() -> Self {
        Self::new()
    }
}

fn init_known_unknown_messages(msg_set: &mut MsgMap) {
    let known = [
        "/cosmos.authz.v1beta1.MsgExec",
        "/cosmos.authz.v1beta1.MsgGrant",
        "/cosmos.authz.v1beta1.MsgRevoke",
        "/cosmos.distribution.v1beta1.MsgFundCommunityPool",
        "/cosmos.distribution.v1beta1.MsgSetWithdrawAddress",
        "/cosmos.distribution.v1beta1.MsgWithdrawDelegatorReward",
        "/cosmos.distribution.v1beta1.MsgWithdrawValidatorCommission",
        "/cosmos.feegrant.v1beta1.MsgGrantAllowance",
        "/cosmos.feegrant.v1beta1.MsgRevokeAllowance",
        "/cosmos.gov.v1beta1.MsgDeposit",
        "/cosmos.gov.v1beta1.MsgSubmitProposal",
        "/cosmos.gov.v1beta1.MsgVote",
        "/cosmos.gov.v1beta1.MsgVoteWeighted",
        "/cosmos.slashing.v1beta1.MsgUnjail",
        "/cosmos.staking.v1beta1.MsgBeginRedelegate",
        "/cosmos.staking.v1beta1.MsgCreateValidator",
        "/cosmos.staking.v1beta1.MsgDelegate",
        "/cosmos.staking.v1beta1.MsgEditValidator",
        "/cosmos.staking.v1beta1.MsgUndelegate",
        "/cosmos.staking.v1beta1.MsgWithdrawDelegatorReward",
        "/cosmos.staking.v1beta1.MsgWithdrawValidatorCommission",
        "/cosmwasm.wasm.v1.MsgMigrateContract",
        "/cosmwasm.wasm.v1.MsgStoreCode",
        "/cosmwasm.wasm.v1.MsgUpdateAdmin",
        "/ibc.applications.transfer.v1.MsgTransfer",
        "/ibc.core.channel.v1.MsgAcknowledgement",
        "/ibc.core.channel.v1.MsgChannelOpenAck",
        "/ibc.core.channel.v1.MsgChannelOpenConfirm",
        "/ibc.core.channel.v1.MsgChannelOpenInit",
        "/ibc.core.channel.v1.MsgChannelOpenTry",
        "/ibc.core.channel.v1.MsgRecvPacket",
        "/ibc.core.channel.v1.MsgTimeout",
        "/ibc.core.client.v1.MsgCreateClient",
        "/ibc.core.client.v1.MsgUpdateClient",
        "/ibc.core.connection.v1.MsgConnectionOpenAck",
        "/ibc.core.connection.v1.MsgConnectionOpenConfirm",
        "/ibc.core.connection.v1.MsgConnectionOpenInit",
        "/ibc.core.connection.v1.MsgConnectionOpenTry",
    ];
    known.map(|msg| msg_set.insert(msg.to_string()));
}
