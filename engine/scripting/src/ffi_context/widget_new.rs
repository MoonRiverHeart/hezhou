use std::ffi::{c_void, c_char};
use super::types::WidgetTreeHandle;

pub type CreateTabWidgetFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32) -> u64;
pub type TabWidgetAddTabFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, u64, bool) -> u32;
pub type TabWidgetSetActiveFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type TabWidgetGetActiveFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type TabWidgetRemoveTabFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type TabWidgetSetOnSelectThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type TabWidgetSetOnCloseThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type TabWidgetGetTabCountFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;

pub type CreateTreeViewFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32) -> u64;
pub type TreeViewAddNodeFn = extern "C" fn(WidgetTreeHandle, u64, u64, *const c_char, u64, bool) -> u64;
pub type TreeViewRemoveNodeFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type TreeViewSetSelectedFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type TreeViewGetSelectedFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type TreeViewExpandNodeFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type TreeViewCollapseNodeFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type TreeViewSetOnSelectThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type TreeNodeSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type TreeNodeGetUserDataFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type TreeViewClearSelectionFn = extern "C" fn(WidgetTreeHandle, u64);

pub type CreatePopupMenuFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type PopupMenuAddItemFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, *const c_char, usize);
pub type PopupMenuAddSeparatorFn = extern "C" fn(WidgetTreeHandle, u64);
pub type PopupMenuShowFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type PopupMenuHideFn = extern "C" fn(WidgetTreeHandle, u64);
pub type PopupMenuIsVisibleFn = extern "C" fn(WidgetTreeHandle, u64) -> bool;
pub type PopupMenuSetOnClickThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

pub type CreateGridViewFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, f32) -> u64;
pub type GridViewAddItemFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, u64) -> u32;
pub type GridViewRemoveItemFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type GridViewSetSelectedFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type GridViewGetSelectedFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type GridViewGetSelectedUserDataFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type GridViewClearFn = extern "C" fn(WidgetTreeHandle, u64);
pub type GridViewItemCountFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type GridViewSetOnClickThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

pub type CreateDialogFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, f32, f32) -> u64;
pub type DialogSetContentFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type DialogAddButtonFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, i32);
pub type DialogShowFn = extern "C" fn(WidgetTreeHandle, u64);
pub type DialogHideFn = extern "C" fn(WidgetTreeHandle, u64);
pub type DialogIsVisibleFn = extern "C" fn(WidgetTreeHandle, u64) -> bool;
pub type DialogGetResultFn = extern "C" fn(WidgetTreeHandle, u64) -> i32;
pub type DialogSetOnResultThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

pub type CreateFileBrowserFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, *const c_char) -> u64;
pub type FileBrowserSetPathFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type FileBrowserSetFilterFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type FileBrowserNavigateUpFn = extern "C" fn(WidgetTreeHandle, u64);
pub type FileBrowserRefreshFn = extern "C" fn(WidgetTreeHandle, u64);
pub type FileBrowserGetSelectedPathFn = extern "C" fn(WidgetTreeHandle, u64, *mut c_char, usize) -> bool;
pub type FileBrowserGetCurrentPathFn = extern "C" fn(WidgetTreeHandle, u64, *mut c_char, usize) -> bool;
pub type FileBrowserSetOnSelectThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type FileBrowserSetOnDoubleClickThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);