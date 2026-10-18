using System;
using System.IO;
using System.Text;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static bool _sampleTextureLoaded;
        private static readonly System.Collections.Generic.Dictionary<string, ulong> _uiTextureCache = new System.Collections.Generic.Dictionary<string, ulong>();

        private static void LoadSampleTextureOnce()
        {
            if (_sampleTextureLoaded) return;
            _sampleTextureLoaded = true;
            try
            {
                string[] candidates = new string[]
                {
                    "assets/textures/gradient.bmp",
                    "engine/assets/textures/gradient.bmp",
                    "../assets/textures/gradient.bmp"
                };
                for (int i = 0; i < candidates.Length; i++)
                {
                    if (!File.Exists(candidates[i])) break;
                    if (!HasTextureAsset("gradient"))
                    {
                        AssetLibrary.LoadTexture(Path.GetFullPath(candidates[i]));
                        Log.Info("Editor", "已加载示例纹理: " + candidates[i]);
                    }
                    break;
                }
                string png = null;
                foreach (string candidate in candidates)
                {
                    string pngPath = candidate.Replace("gradient.bmp", "spectrum.png");
                    if (File.Exists(pngPath)) { png = pngPath; break; }
                }
                if (png != null && !HasTextureAsset("spectrum"))
                {
                    AssetLibrary.LoadTexture(Path.GetFullPath(png));
                    Log.Info("Editor", "已加载 PNG 纹理: " + png);
                }
            }
            catch (Exception ex)
            {
                Log.Warn("Editor", "加载示例纹理失败: " + ex.Message);
            }
        }

        private static bool HasTextureAsset(string name)
        {
            int categoryCount = AssetLibrary.GetCategoryCount();
            for (int c = 0; c < categoryCount; c++)
            {
                string category = AssetLibrary.GetCategoryName(c);
                if (category == null) continue;
                if (category.IndexOf("Texture", StringComparison.OrdinalIgnoreCase) < 0
                    && category.IndexOf("纹理", StringComparison.Ordinal) < 0) continue;
                int count = AssetLibrary.GetAssetCount(c);
                for (int i = 0; i < count; i++)
                {
                    AssetInfo info = AssetLibrary.GetAssetInfo(c, i);
                    if (info.Name == name) return true;
                }
            }
            return false;
        }

        private static ulong LoadUiTextureId(string path)
        {
            if (string.IsNullOrEmpty(path)) return 0;
            ulong id;
            if (_uiTextureCache.TryGetValue(path, out id)) return id;
            id = UI.LoadImageTexture(path);
            if (id != 0) _uiTextureCache[path] = id;
            return id;
        }

        /// <summary>把 BMP 降到 8x8，输出 pixels:RRGGBB... 供 GridView 画像素缩略图。</summary>
        public static bool TestHasGradientTexture() { return HasTextureAsset("gradient"); }
        public static bool TestHasPngTexture() { return HasTextureAsset("spectrum"); }

        private static string DecodeBmpThumbnail(string path)
        {
            try
            {
                if (string.IsNullOrEmpty(path) || !File.Exists(path)) return null;
                byte[] data = File.ReadAllBytes(path);
                if (data.Length < 54 || data[0] != (byte)'B' || data[1] != (byte)'M') return null;
                int pixelOffset = BitConverter.ToInt32(data, 10);
                int width = BitConverter.ToInt32(data, 18);
                int height = BitConverter.ToInt32(data, 22);
                int bpp = BitConverter.ToInt16(data, 28);
                if (width <= 0 || height <= 0 || (bpp != 24 && bpp != 32)) return null;
                int bytesPerPixel = bpp / 8;
                int rowSize = ((width * bpp + 31) / 32) * 4;
                StringBuilder sb = new StringBuilder("pixels:");
                for (int ty = 0; ty < 8; ty++)
                {
                    int sy = height - 1 - (ty * (height - 1) / 7);
                    for (int tx = 0; tx < 8; tx++)
                    {
                        int sx = tx * (width - 1) / 7;
                        int idx = pixelOffset + sy * rowSize + sx * bytesPerPixel;
                        if (idx + 2 >= data.Length) return null;
                        byte b = data[idx];
                        byte g = data[idx + 1];
                        byte r = data[idx + 2];
                        sb.Append(r.ToString("X2")).Append(g.ToString("X2")).Append(b.ToString("X2"));
                    }
                }
                return sb.ToString();
            }
            catch (Exception)
            {
                return null;
            }
        }
    }
}
