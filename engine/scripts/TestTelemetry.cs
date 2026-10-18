using System;
using System.Collections.Generic;
using System.Text;

namespace Hezhou
{
    /// <summary>
    /// 自动化测试期间的关键日志/错误采集器。
    /// 只收集 Warn/Error/Fatal，避免报告被普通 Info 日志淹没。
    /// 不依赖 System.Text.Json（mcs 兼容），手写 JSON 转义。
    /// </summary>
    public static class TestTelemetry
    {
        public class Entry
        {
            public string Timestamp;
            public string Level;
            public string Module;
            public string Message;
        }

        private static readonly object _lock = new object();
        private static List<Entry> _entries = new List<Entry>();
        private static List<Entry> _allEntries = new List<Entry>();
        private static int _errorCount;
        private static int _warnCount;
        private static int _fatalCount;

        public static int WarningCount
        {
            get { lock (_lock) { return _warnCount; } }
        }

        public static int ErrorCount
        {
            get { lock (_lock) { return _errorCount; } }
        }

        public static int FatalCount
        {
            get { lock (_lock) { return _fatalCount; } }
        }

        public static int TotalCount
        {
            get { lock (_lock) { return _entries.Count; } }
        }

        public static void Clear()
        {
            lock (_lock)
            {
                _entries.Clear();
                _allEntries.Clear();
                _errorCount = 0;
                _warnCount = 0;
                _fatalCount = 0;
            }
        }

        /// <summary>
        /// 记录全部级别日志，供 Console 面板显示。最多保留 500 条。
        /// </summary>
        public static void RecordAll(LogLevel level, string module, string message)
        {
            Entry entry = new Entry();
            entry.Timestamp = DateTime.Now.ToString("HH:mm:ss.fff");
            entry.Level = level.ToString();
            entry.Module = module ?? "";
            entry.Message = message ?? "";

            lock (_lock)
            {
                _allEntries.Add(entry);
                if (_allEntries.Count > 500)
                {
                    _allEntries.RemoveAt(0);
                }
            }
        }

        public static string GetRecentLogs(int maxLines)
        {
            if (maxLines <= 0) maxLines = 200;

            lock (_lock)
            {
                StringBuilder sb = new StringBuilder();
                int start = _allEntries.Count > maxLines ? _allEntries.Count - maxLines : 0;
                for (int i = start; i < _allEntries.Count; i++)
                {
                    Entry e = _allEntries[i];
                    sb.Append(e.Timestamp).Append(" [").Append(e.Level).Append("] [")
                      .Append(e.Module).Append("] ").AppendLine(e.Message);
                }
                return sb.ToString();
            }
        }

        public static void Record(LogLevel level, string module, string message)
        {
            if (level < LogLevel.Warn)
            {
                return;
            }

            Entry entry = new Entry();
            entry.Timestamp = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss.fff");
            entry.Level = level.ToString();
            entry.Module = module ?? "";
            entry.Message = message ?? "";

            lock (_lock)
            {
                _entries.Add(entry);

                // 限制内存占用，防止长时间压力测试无限增长。
                if (_entries.Count > 500)
                {
                    _entries.RemoveAt(0);
                }

                if (level == LogLevel.Warn) _warnCount++;
                else if (level == LogLevel.Error) _errorCount++;
                else if (level == LogLevel.Fatal) _fatalCount++;
            }
        }

        public static void RecordError(string module, string message)
        {
            Record(LogLevel.Error, module, message);
        }

        public static string ToJsonArray()
        {
            lock (_lock)
            {
                StringBuilder sb = new StringBuilder();
                sb.Append("[");
                for (int i = 0; i < _entries.Count; i++)
                {
                    if (i > 0) sb.Append(",");
                    Entry e = _entries[i];
                    sb.Append("{");
                    sb.Append("\"time\":").Append(JsonString(e.Timestamp)).Append(",");
                    sb.Append("\"level\":").Append(JsonString(e.Level)).Append(",");
                    sb.Append("\"module\":").Append(JsonString(e.Module)).Append(",");
                    sb.Append("\"message\":").Append(JsonString(e.Message));
                    sb.Append("}");
                }
                sb.Append("]");
                return sb.ToString();
            }
        }

        public static string BuildSummary()
        {
            lock (_lock)
            {
                return "warnings=" + _warnCount + ", errors=" + _errorCount + ", fatal=" + _fatalCount;
            }
        }

        public static string JsonString(string value)
        {
            if (value == null) return "null";

            StringBuilder sb = new StringBuilder();
            sb.Append("\"");
            for (int i = 0; i < value.Length; i++)
            {
                char c = value[i];
                switch (c)
                {
                    case '\\': sb.Append("\\\\"); break;
                    case '"': sb.Append("\\\""); break;
                    case '\n': sb.Append("\\n"); break;
                    case '\r': sb.Append("\\r"); break;
                    case '\t': sb.Append("\\t"); break;
                    default:
                        if (c < 0x20)
                        {
                            sb.Append("\\u").Append(((int)c).ToString("x4"));
                        }
                        else
                        {
                            sb.Append(c);
                        }
                        break;
                }
            }
            sb.Append("\"");
            return sb.ToString();
        }
    }
}
