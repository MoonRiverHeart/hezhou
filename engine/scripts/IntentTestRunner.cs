using System;

namespace Hezhou
{
    /// <summary>
    /// 意图级功能测试：把用户意图（新建场景/创建实体/选择/删除/运行暂停）
    /// 翻译为可重复的编辑器操作，并复用 UITestRunner 的报告与错误采集。
    ///
    /// AI 新增意图时，优先在 RunAll() 中增加一个语义化 case；
    /// 每个 case 只通过 EditorScript.Test* 钩子操作，不依赖具体 widget id。
    /// </summary>
    public static class IntentTestRunner
    {
        public static void RunAll()
        {
            RunCase("intent.new_scene_defaults", NewSceneDefaults);
            RunCase("intent.create_select_delete_entity", CreateSelectDeleteEntity);
            RunCase("intent.parent_child_hierarchy", ParentChildHierarchy);
            RunCase("intent.undo_redo_property", UndoRedoProperty);
            RunCase("intent.multi_select_batch", MultiSelectBatch);
            RunCase("intent.bind_rotation_script", BindRotationScript);
            RunCase("intent.icon_system", IconSystem);
            RunCase("intent.console_open_close", ConsoleOpenClose);
            RunCase("intent.running_paused_editing", RunningPausedEditing);
        }

        private static void RunCase(string name, Func<string> body)
        {
            try
            {
                string detail = body();
                UITestRunner.RecordIntentStep(name, true, detail);
            }
            catch (Exception ex)
            {
                UITestRunner.RecordIntentStep(name, false, ex.Message);
            }
        }

        private static string NewSceneDefaults()
        {
            EditorScript.TestNewScene();
            int count = EditorScript.TestGetEntityCount();
            if (count != 3)
            {
                throw new Exception("expected 3 default entities, actual=" + count);
            }
            if (EditorScript.TestGetGameState() != 0)
            {
                throw new Exception("new scene should be in Editing state");
            }
            return "entities=3, state=Editing";
        }

        private static string CreateSelectDeleteEntity()
        {
            int before = EditorScript.TestGetEntityCount();
            ulong entityId = EditorScript.TestCreateMeshEntity(0);
            if (entityId == 0)
            {
                throw new Exception("CreateMeshEntity returned 0");
            }
            if (EditorScript.TestGetEntityCount() != before + 1)
            {
                throw new Exception("entity count did not increase");
            }
            if (!EditorScript.TestSelectEntity(entityId))
            {
                throw new Exception("entity selection failed");
            }
            if (EditorScript.TestGetSelectedEntityId() != entityId)
            {
                throw new Exception("selected id mismatch");
            }
            if (!EditorScript.TestDeleteSelectedEntity())
            {
                throw new Exception("delete selected entity failed");
            }
            if (EditorScript.TestGetEntityCount() != before)
            {
                throw new Exception("delete did not restore entity count");
            }
            return "created, selected and deleted entity " + entityId;
        }

        private static string ParentChildHierarchy()
        {
            EditorScript.TestNewScene();
            ulong parent = EditorScript.TestCreateMeshEntity(0);
            ulong child = EditorScript.TestCreateMeshEntity(0);
            if (parent == 0 || child == 0)
            {
                throw new Exception("failed to create parent/child entities");
            }

            if (!EditorScript.TestSetParent(child, parent))
            {
                throw new Exception("failed to set parent");
            }
            if (EditorScript.TestGetParent(child) != parent)
            {
                throw new Exception("parent id mismatch");
            }
            if (EditorScript.TestGetChildCount(parent) != 1)
            {
                throw new Exception("child count mismatch");
            }

            // 自环/祖先环必须被拒绝。
            if (EditorScript.TestSetParent(parent, child))
            {
                throw new Exception("cycle was accepted");
            }

            if (!EditorScript.TestSetParent(child, 0))
            {
                throw new Exception("failed to detach child");
            }
            return "parent/child attached, cycle rejected, child detached";
        }

        private static string UndoRedoProperty()
        {
            EditorScript.TestNewScene();
            ulong entityId = EditorScript.TestCreateMeshEntity(0);
            if (entityId == 0 || !EditorScript.TestSelectEntity(entityId))
            {
                throw new Exception("failed to create/select entity for undo test");
            }

            EditorScript.TestCaptureUndo();
            if (!EditorScript.TestSetPosition(entityId, 7f, 8f, 9f))
            {
                throw new Exception("failed to set position");
            }

            EditorScript.TestUndo();
            float[] afterUndo = EditorScript.TestGetPosition(entityId);
            if (Math.Abs(afterUndo[0] - 7f) < 0.001f)
            {
                throw new Exception("undo did not restore position");
            }

            EditorScript.TestRedo();
            float[] afterRedo = EditorScript.TestGetPosition(entityId);
            if (Math.Abs(afterRedo[0] - 7f) > 0.001f
                || Math.Abs(afterRedo[1] - 8f) > 0.001f
                || Math.Abs(afterRedo[2] - 9f) > 0.001f)
            {
                throw new Exception("redo did not restore position");
            }

            return "position 7,8,9 -> undo -> redo";
        }

        private static string MultiSelectBatch()
        {
            EditorScript.TestNewScene();
            ulong first = EditorScript.TestCreateMeshEntity(0);
            ulong second = EditorScript.TestCreateMeshEntity(0);
            if (first == 0 || second == 0)
            {
                throw new Exception("failed to create entities");
            }

            EditorScript.TestSelectEntity(first);
            EditorScript.TestToggleEntitySelection(second);
            if (EditorScript.TestGetSelectionCount() != 2)
            {
                throw new Exception("multi-select count mismatch");
            }

            EditorScript.TestBatchSetPositionX(42f);
            float[] p1 = EditorScript.TestGetPosition(first);
            float[] p2 = EditorScript.TestGetPosition(second);
            if (Math.Abs(p1[0] - 42f) > 0.001f || Math.Abs(p2[0] - 42f) > 0.001f)
            {
                throw new Exception("batch property edit did not apply to both entities");
            }

            int beforeDelete = EditorScript.TestGetEntityCount();
            if (!EditorScript.TestDeleteSelectedEntity())
            {
                throw new Exception("multi-select delete failed");
            }
            if (EditorScript.TestGetEntityCount() != beforeDelete - 2)
            {
                throw new Exception("multi-select delete did not remove both entities");
            }
            return "2 entities selected, batch edit + multi-delete passed";
        }

        private static string IconSystem()
        {
            uint count = UI.IconCount();
            if (count < 20) throw new Exception("expected icon set, actual=" + count);
            return "icon count=" + count;
        }

        private static string BindRotationScript()
        {
            EditorScript.TestNewScene();
            ulong entityId = EditorScript.TestCreateMeshEntity(0);
            if (entityId == 0 || !EditorScript.TestSelectEntity(entityId))
            {
                throw new Exception("failed to prepare entity for script binding");
            }

            // 走真实 UI 按钮路径：选择下拉项 → 点击“绑定”
            EditorScript.TestSetPropertyTab(3);
            EditorScript.TestSelectScript("RotationScript.cs");
            EditorScript.TestClickBindButton();

            string className = EditorScript.TestGetLastBindingClass();
            if (className != "RotationScript")
            {
                throw new Exception("expected className RotationScript, actual=" + className);
            }
            return "RotationScript bound to entity " + entityId;
        }

        private static string ConsoleOpenClose()
        {
            if (!EditorScript.TestOpenConsole())
            {
                throw new Exception("console dialog did not open");
            }
            EditorScript.TestCloseConsole();
            return "console opened and closed";
        }

        private static string RunningPausedEditing()
        {
            if (!EditorScript.TestSetGameState(1) || EditorScript.TestGetGameState() != 1)
            {
                throw new Exception("failed to enter Running");
            }
            if (!EditorScript.TestSetGameState(2) || EditorScript.TestGetGameState() != 2)
            {
                throw new Exception("failed to enter Paused");
            }
            if (!EditorScript.TestSetGameState(0) || EditorScript.TestGetGameState() != 0)
            {
                throw new Exception("failed to return to Editing");
            }
            return "Editing -> Running -> Paused -> Editing";
        }
    }
}
