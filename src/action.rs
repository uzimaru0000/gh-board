#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Universal
    Quit,
    ForceQuit,
    Back,

    // Navigation
    MoveDown,
    MoveUp,
    MoveLeft,
    MoveRight,
    FirstItem,
    LastItem,
    NextTab,
    PrevTab,

    // Board
    OpenDetail,
    GrabCard,
    NewCard,
    ConvertDraftToIssue,
    ArchiveCard,
    ShowArchivedList,
    StartFilter,
    ClearFilter,
    Refresh,
    ShowHelp,
    SwitchProject,
    ChangeGrouping,
    ToggleLayout,

    // Detail content
    OpenInBrowser,
    EditCard,
    NewComment,
    OpenCommentList,
    CopyUrl,

    // Detail sidebar / confirm / grab / forms
    Select,
    ConfirmYes,
    ConfirmNo,
    ConfirmGrab,
    CancelGrab,
    EditComment,
    Submit,
    NextField,
    PrevField,
    ToggleType,
    OpenEditor,
    ToggleItem,

    // Reactions
    OpenReactionPicker,
    ToggleReaction,

    // Bulk selection
    BulkSelectStart,
    BulkSelectToggle,
    BulkSelectAll,
    BulkSelectClear,
    BulkArchive,
    BulkMoveLeft,
    BulkMoveRight,

    // Custom commands
    /// `[[command]]` で定義された config index 番目のコマンドを発火する。
    /// パレット経由でも同じ発火に最終的に行き着く。
    RunCustomCommand(u8),
    /// コマンドパレット (`:`) を開く。
    OpenCommandPalette,
}
