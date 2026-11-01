# 音频测试资源

## `tone_440hz_100ms.ogg`

- 内容：0.1 秒、440Hz、48kHz 单声道正弦波（OGG/Vorbis），约 4KB。
- 用途：`--features ogg` 下 lewton 解码器与引擎 OGG 加载链路的真实文件测试。
- 生成方式：numpy 生成正弦波 + libsndfile（Python `soundfile`）编码为 OGG/Vorbis，
  由本项目自行生成，不含任何第三方录音/版权素材。
- 重新生成脚本（需 `pip install soundfile`）：

```python
import numpy as np
import soundfile as sf

sr, duration, freq = 48000, 0.1, 440.0
t = np.arange(int(duration * sr)) / sr
samples = (0.25 * np.sin(2 * np.pi * freq * t)).astype(np.float32)
sf.write("engine/audio/tests/assets/tone_440hz_100ms.ogg", samples, sr,
         format="OGG", subtype="VORBIS")
```

> 解码后帧数可能略多于 4800（Vorbis 编码延迟/尾部填充），测试对帧数采用区间断言。
