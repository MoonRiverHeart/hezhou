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
pub type TreeViewSetOnToggleThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type TreeViewIsNodeExpandedFn = extern "C" fn(WidgetTreeHandle, u64, u64) -> bool;
pub type TreeNodeSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type TreeNodeSetSelectedFn = extern "C" fn(WidgetTreeHandle, u64, bool);
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
pub type GridViewSetItemPaddingFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);

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

// Slider
pub type CreateSliderFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32) -> u64;
pub type CreateSliderInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, f32, f32, f32) -> u64;
pub type SliderSetValueFn = extern "C" fn(WidgetTreeHandle, u64, f32);
pub type SliderGetValueFn = extern "C" fn(WidgetTreeHandle, u64) -> f32;
pub type SliderSetRangeFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type SliderSetOnChangeThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type SliderSetStepFn = extern "C" fn(WidgetTreeHandle, u64, f32);

// ScrollView
pub type CreateScrollViewFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32) -> u64;
pub type ScrollViewSetScrollOffsetFn = extern "C" fn(WidgetTreeHandle, u64, f32);
pub type ScrollViewGetScrollOffsetFn = extern "C" fn(WidgetTreeHandle, u64) -> f32;
pub type ScrollViewSetShowScrollbarsFn = extern "C" fn(WidgetTreeHandle, u64, u32, u32);
pub type ScrollViewSetContentSizeFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type ScrollViewSetOnScrollThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

// SplitView
pub type CreateSplitViewFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, u32) -> u64;
pub type SplitViewSetSplitRatioFn = extern "C" fn(WidgetTreeHandle, u64, f32);
pub type SplitViewGetSplitRatioFn = extern "C" fn(WidgetTreeHandle, u64) -> f32;
pub type SplitViewSetMinRatioFn = extern "C" fn(WidgetTreeHandle, u64, f32);
pub type SplitViewSetMaxRatioFn = extern "C" fn(WidgetTreeHandle, u64, f32);
pub type SplitViewSetOnRatioChangeThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

// Image
pub type CreateImageFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32) -> u64;
pub type ImageSetTextureIdFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type ImageGetTextureIdFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type ImageSetScaleModeFn = extern "C" fn(WidgetTreeHandle, u64, u32);
pub type ImageSetUvFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32);