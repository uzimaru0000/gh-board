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
    MoveCardLeft,
    MoveCardRight,
    GrabCard,
    NewCard,
    DeleteCard,
    StartFilter,
    ClearFilter,
    Refresh,
    ShowHelp,
    SwitchProject,

    // Detail content
    OpenInBrowser,
    EditCard,
    NewComment,
    OpenCommentList,

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
}
