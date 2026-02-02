use bytes::Buf;
use prost::Message;

pub mod dota {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// Some messages we don't get from the provided proto files, this is a placeholder message
// to indicate that we don't have the ability to decode these yet

#[derive(Debug)]
pub struct UnimplementedMessage;

impl UnimplementedMessage {
    fn decode<B: Buf>(_buf: B) -> Result<UnimplementedMessage> {
        Ok(UnimplementedMessage {})
    }
}

macro_rules! decoder {
    ($enum_name:ident, $enum_kind_name:ident, $( [$id:expr, $name:ident, $message:ty] ),*) => {
        #[derive(Debug)]
        pub enum $enum_name {
            $(
                $name($message),
            )*
        }

        #[repr(u32)]
        #[derive(Eq, Hash, PartialEq, Debug, Clone)]
        pub enum $enum_kind_name {
            $(
                $name = $id,
            )*
        }

        impl TryFrom<u32> for $enum_kind_name {
            type Error = Box<dyn std::error::Error>;

            fn try_from(v: u32) -> Result<Self> {
                match v {
                    $(
                        x if x == $enum_kind_name::$name as u32 => Ok($enum_kind_name::$name),
                    )*
                    _ => Err(format!("invalid {:} kind: {:?}", stringify!($enum_name), v).into()),
                }
            }
        }

        impl $enum_name {
            pub fn decode<B>(kind: &$enum_kind_name, buf: B) -> Result<$enum_name>
                where B: Buf {
                match kind {
                    $(
                        $enum_kind_name::$name => {
                            use $message as message;
                            Ok($enum_name::$name(message::decode(buf)?))
                        },
                    )*
                }
            }
        }
    };
}

decoder!(
    Demo,
    DemoKind,
    [0, Stop, dota::CDemoStop],
    [1, Header, dota::CDemoFileHeader],
    [2, FileInfo, dota::CDemoFileInfo],
    [3, SyncTick, dota::CDemoSyncTick],
    [4, SendTables, dota::CDemoSendTables],
    [5, ClassInfo, dota::CDemoClassInfo],
    [6, StringTables, dota::CDemoStringTables],
    [7, Packet, dota::CDemoPacket],
    [8, SignOnPacket, dota::CDemoPacket],
    [13, FullPacket, dota::CDemoFullPacket],
    [14, SaveGame, dota::CDemoSaveGame],
    [15, SpawnGroups, dota::CDemoSpawnGroups],
    [16, AnimationData, dota::CDemoAnimationData],
    [17, AnimationHeader, dota::CDemoAnimationHeader],
    [18, Recovery, dota::CDemoRecovery]
);

decoder!(
    Packet,
    PacketKind,
    // NET_Messages
    [0, NetNop, dota::CnetMsgNop],
    [4, Tick, dota::CnetMsgTick],
    [6, SetConVar, dota::CnetMsgSetConVar],
    [7, SignonState, dota::CnetMsgSignonState],
    [8, SpawnGroupLoad, dota::CnetMsgSpawnGroupLoad],
    [
        9,
        SpawnGroupManifestUpdate,
        dota::CnetMsgSpawnGroupManifestUpdate
    ],
    [
        11,
        SpawnGroupSetCreationTick,
        dota::CnetMsgSpawnGroupSetCreationTick
    ],
    [12, SpawnGroupUnload, dota::CnetMsgSpawnGroupUnload],
    [
        13,
        SpawnGroupLoadCompleted,
        dota::CnetMsgSpawnGroupLoadCompleted
    ],
    // CLC_Messages
    [20, ClientInfo, dota::CclcMsgClientInfo],
    [21, Move, dota::CclcMsgMove],
    [23, BaselineAck, dota::CclcMsgBaselineAck],
    [27, LoadingProgress, dota::CclcMsgLoadingProgress],
    // SVC_Messages
    [40, ServerInfo, dota::CsvcMsgServerInfo],
    [41, FlattenedSerializer, dota::CsvcMsgFlattenedSerializer],
    [42, ClassInfo, dota::CsvcMsgClassInfo],
    [44, CreateStringTable, dota::CsvcMsgCreateStringTable],
    [45, UpdateStringTable, dota::CsvcMsgUpdateStringTable],
    [46, VoiceInit, dota::CsvcMsgVoiceInit],
    [47, VoiceData, dota::CclcMsgVoiceData],
    [48, Print, dota::CsvcMsgPrint],
    [50, SetView, dota::CsvcMsgSetView],
    [51, ClearAllStringTables, dota::CsvcMsgClearAllStringTables],
    [55, PacketEntities, dota::CsvcMsgPacketEntities],
    [60, PeerList, dota::CsvcMsgPeerList],
    [62, HltvStatus, dota::CsvcMsgHltvStatus],
    [70, FullFrameSplit, dota::CsvcMsgFullFrameSplit],
    // EBaseUserMessages
    [106, Fade, dota::CUserMessageFade],
    [115, ResetHud, dota::CUserMessageResetHud],
    [118, SayText2, dota::CUserMessageSayText2],
    [124, TextMsg, dota::CUserMessageTextMsg],
    [128, VoiceMask, dota::CUserMessageVoiceMask],
    [130, SendAudio, dota::CUserMessageSendAudio],
    [144, AudioParameter, dota::CUserMessageAudioParameter],
    //[145, ParticleManager, dota::CdotaUserMsgParticleManager],
    [145, ParticleManager145, UnimplementedMessage],
    // EBaseGameEvents
    [205, Source1LegacyGameEventList, dota::CsvcMsgGameEventList],
    [
        206,
        Source1LegacyListenEvents,
        dota::CMsgSource1LegacyListenEvents
    ],
    [207, Source1LegacyGameEvent, dota::CsvcMsgGameEvent],
    [208, SosStartSoundEvent, dota::CMsgSosStartSoundEvent],
    [209, SosStopSoundEvent, dota::CMsgSosStopSoundEvent],
    [
        210,
        SosSetSoundEventParams,
        dota::CMsgSosSetSoundEventParams
    ],
    [212, SosStopSoundEventHash, dota::CMsgSosStopSoundEventHash],
    // ETEProtobufIds
    //[400, EffectDispatch, s2te::CMsgTEEffectDispatch]
    [400, EffectDispatch, UnimplementedMessage],
    // EDotaUserMessages
    [466, ChatEvent, dota::CdotaUserMsgChatEvent],
    [470, CombatLogBulkData, dota::CdotaUserMsgCombatLogBulkData],
    [
        471,
        CreateLinearProjectile,
        dota::CdotaUserMsgCreateLinearProjectile
    ],
    [
        472,
        DestroyLinearProjectile,
        dota::CdotaUserMsgDestroyLinearProjectile
    ],
    [474, GlobalLightColor, dota::CdotaUserMsgGlobalLightColor],
    [
        475,
        GlobalLightDirection,
        dota::CdotaUserMsgGlobalLightDirection
    ],
    [
        473,
        DodgeTrackingProjectiles,
        dota::CdotaUserMsgDodgeTrackingProjectiles
    ],
    [477, LocationPing, dota::CdotaUserMsgLocationPing],
    [478, MapLine, dota::CdotaUserMsgMapLine],
    [479, MiniKillCamInfo, dota::CdotaUserMsgMiniKillCamInfo],
    [481, MinimapEvent, dota::CdotaUserMsgMinimapEvent],
    [482, NevermoreRequiem, dota::CdotaUserMsgNevermoreRequiem],
    [483, OverheadEvent, dota::CdotaUserMsgOverheadEvent],
    [485, SharedCooldown, dota::CdotaUserMsgSharedCooldown],
    [
        486,
        SpectatorPlayerClick,
        dota::CdotaUserMsgSpectatorPlayerClick
    ],
    [488, UnitEvent, dota::CdotaUserMsgUnitEvent],
    // [
    //     489,
    //     ParticleManager,
    //     dota::CdotaUserMsgParticleManager
    //],
    [489, ParticleManager489, UnimplementedMessage],
    [490, BotChat, dota::CdotaUserMsgBotChat],
    [491, HudError, dota::CdotaUserMsgHudError],
    [492, ItemPurchased, dota::CdotaUserMsgItemPurchased],
    [497, WorldLine, dota::CdotaUserMsgWorldLine],
    [501, ChatWheel, dota::CdotaUserMsgChatWheel],
    [
        506,
        GamerulesStateChanged,
        dota::CdotaUserMsgGamerulesStateChanged
    ],
    [510, SendStatPopup, dota::CdotaUserMsgSendStatPopup],
    [512, SendRoshanPopup, dota::CdotaUserMsgSendRoshanPopup],
    [518, Projectile, dota::CdotaUserMsgTeProjectile],
    [519, ProjectileLoc, dota::CdotaUserMsgTeProjectileLoc],
    [520, DotaBloodImpact, dota::CdotaUserMsgTeDotaBloodImpact],
    [521, UnitAnimation, dota::CdotaUserMsgTeUnitAnimation],
    [522, UnitAnimationEnd, dota::CdotaUserMsgTeUnitAnimationEnd],
    [523, AbilityPing, dota::CdotaUserMsgAbilityPing],
    [529, WillPurchaseAlert, dota::CdotaUserMsgWillPurchaseAlert],
    [532, AbilitySteal, dota::CdotaUserMsgAbilitySteal],
    [
        533,
        CourierKilledAlert,
        dota::CdotaUserMsgCourierKilledAlert
    ],
    [534, EnemyItemAlert, dota::CdotaUserMsgEnemyItemAlert],
    [540, QuickBuyAlert, dota::CdotaUserMsgQuickBuyAlert],
    [543, ModifierAlert, dota::CdotaUserMsgModifierAlert],
    [544, HPManaAlert, dota::CdotaUserMsgHpManaAlert],
    [
        547,
        SpectatorPlayerUnitOrders,
        dota::CdotaUserMsgSpectatorPlayerUnitOrders
    ],
    [552, ProjectionAbility, dota::CdotaUserMsgProjectionAbility],
    [553, ProjectionEvent, dota::CdotaUserMsgProjectionEvent],
    [554, CombatLogData, dota::CMsgDotaCombatLogEntry],
    [555, XpAlert, dota::CdotaUserMsgXpAlert],
    [
        556,
        UpdateQuestProgress,
        dota::CdotaUserMsgUpdateQuestProgress
    ],
    // [
    //     557,
    //     MatchMetadata,
    //     S2DotaMatchMetadata::CdotaMatchMetadataFile
    // ],
    [557, MatchMetadata, UnimplementedMessage],
    //[558, MatchDetails, S2DotaGcCommon::CMsgDOTAMatch],
    [558, MatchDetails, UnimplementedMessage],
    [563, SelectPenaltyGold, dota::CdotaUserMsgSelectPenaltyGold],
    [564, RollDiceResult, dota::CdotaUserMsgRollDiceResult],
    [565, FlipCoinResult, dota::CdotaUserMsgFlipCoinResult],
    [
        567,
        TeamCaptainChanged,
        dota::CdotaUserMessageTeamCaptainChanged
    ],
    [
        568,
        SendRoshanSpectatorPhase,
        dota::CdotaUserMsgSendRoshanSpectatorPhase
    ],
    [
        571,
        DestroyProjectile,
        dota::CdotaUserMsgTeDestroyProjectile
    ],
    [572, HeroRelicProgress, dota::CdotaUserMsgHeroRelicProgress],
    [574, ItemSold, dota::CdotaUserMsgItemSold],
    [575, DamageReport, dota::CdotaUserMsgDamageReport],
    [576, SalutePlayer, dota::CdotaUserMsgSalutePlayer],
    [577, TipAlert, dota::CdotaUserMsgTipAlert],
    [
        579,
        EmptyTeleportAlert,
        dota::CdotaUserMsgEmptyTeleportAlert
    ],
    [
        580,
        MarsArenaOfBloodAttack,
        dota::CdotaUserMsgMarsArenaOfBloodAttack
    ],
    [581, EsArcanaCombo, dota::CdotaUserMsgEsArcanaCombo],
    [
        582,
        EsArcanaComboSummary,
        dota::CdotaUserMsgEsArcanaComboSummary
    ],
    [
        583,
        HighFiveLeftHanging,
        dota::CdotaUserMsgHighFiveLeftHanging
    ],
    [584, HighFiveCompleted, dota::CdotaUserMsgHighFiveCompleted],
    [585, ShovelUnearth, dota::CdotaUserMsgShovelUnearth],
    [587, RadarAlert, dota::CdotaUserMsgRadarAlert],
    [588, AllStarEvent, dota::CdotaUserMsgAllStarEvent],
    [589, TalentTreeAlert, dota::CdotaUserMsgTalentTreeAlert],
    [
        590,
        QueuedOrderRemoved,
        dota::CdotaUserMsgQueuedOrderRemoved
    ],
    [591, DebugChallenge, dota::CdotaUserMsgDebugChallenge],
    [592, OMArcanaCombo, dota::CdotaUserMsgOmArcanaCombo],
    [593, FoundNeutralItem, dota::CdotaUserMsgFoundNeutralItem],
    [594, OutpostCaptured, dota::CdotaUserMsgOutpostCaptured],
    [595, OutpostGrantedXp, dota::CdotaUserMsgOutpostGrantedXp],
    [596, MoveCameraToUnit, dota::CdotaUserMsgMoveCameraToUnit],
    [597, PauseMinigameData, dota::CdotaUserMsgPauseMinigameData],
    [
        598,
        VersusScenePlayerBehavior,
        dota::CdotaUserMsgVersusScenePlayerBehavior
    ],
    [600, QopArcanaSummary, dota::CdotaUserMsgQoPArcanaSummary],
    [601, HotPotatoCreated, dota::CdotaUserMsgHotPotatoCreated],
    [602, HotPotatoExploded, dota::CdotaUserMsgHotPotatoExploded],
    [603, WkArcanaProgress, dota::CdotaUserMsgWkArcanaProgress],
    [
        604,
        GuildChallengeProgress,
        dota::CdotaUserMsgGuildChallengeProgress
    ],
    [605, WrArcanaProgress, dota::CdotaUserMsgWrArcanaProgress],
    [606, WrArcanaSummary, dota::CdotaUserMsgWrArcanaSummary],
    [612, ChatMessage, dota::CdotaUserMsgChatMessage],
    [613, NeutralCampAlert, dota::CdotaUserMsgNeutralCampAlert],
    [
        614,
        RockPaperScissorsStarted,
        dota::CdotaUserMsgRockPaperScissorsStarted
    ],
    [
        615,
        RockPaperScissorsFinished,
        dota::CdotaUserMsgRockPaperScissorsFinished
    ],
    [
        616,
        DuelOpponentKilled,
        dota::CdotaUserMsgDuelOpponentKilled
    ],
    [617, DuelAccepted, dota::CdotaUserMsgDuelAccepted],
    [618, DuelRequested, dota::CdotaUserMsgDuelRequested],
    [
        619,
        MuertaReleaseEventAssignedTargetKilled,
        dota::CdotaUserMsgMuertaReleaseEventAssignedTargetKilled
    ],
    [
        620,
        PlayerDraftSuggestPick,
        dota::CdotaUserMsgPlayerDraftSuggestPick
    ],
    [621, PlayerDraftPick, dota::CdotaUserMsgPlayerDraftPick],
    [624, FacetPing, dota::CdotaUserMsgFacetPing],
    [625, InnatePing, dota::CdotaUserMsgInnatePing],
    [626, RoshanTimer, dota::CdotaUserMsgRoshanTimer],
    [
        627,
        NeutralCraftAvailable,
        dota::CdotaUserMsgNeutralCraftAvailable
    ],
    [628, TimerAlert, dota::CdotaUserMsgTimerAlert],
    [629, MadstoneAlert, dota::CdotaUserMsgMadstoneAlert],
    [
        630,
        CourierLeftFountainAlert,
        dota::CdotaUserMsgCourierLeftFountainAlert
    ],
    [
        631,
        MonsterHunterInvestigationsAvailable,
        dota::CdotaUserMsgMonsterHunterInvestigationsAvailable
    ],
    [
        632,
        MonsterHunterInvestigationGameState,
        dota::CdotaUserMsgMonsterHunterInvestigationGameState
    ],
    [
        633,
        MonsterHunterHuntAlert,
        dota::CdotaUserMsgMonsterHunterHuntAlert
    ],
    [634, TormentorTimer, dota::CdotaUserMsgTormentorTimer],
    [635, KillEffect, dota::CdotaUserMsgKillEffect]
);
