# haiti-lite-rust

一个独立的 Windows x64 hash 类型识别工具。

本程序参考项目 [noraj/haiti](https://github.com/noraj/haiti) 中 `haiti-parsable` 的核心识别逻辑进行开发，面向程序调用，默认输出 JSON。

## 使用前准备

程序不包含规则数据，使用前需要从原始 haiti 项目取得 `data` 目录，并确认其中有以下两个文件：

```text
data\
  prototypes.json
  commons.json
```

建议目录结构：

```text
haiti-lite-rust\
  haiti-lite-rust.exe
  data\
    prototypes.json
    commons.json
```

程序只读取这两个 JSON 文件，不会复制、解压或修改它们。替换规则文件后下次运行会直接使用新数据。

## 命令用法

### Hashcat 模式

```powershell
.\haiti-lite-rust.exe --data-dir ".\data" hc 5f4dcc3b5aa765d61d8327deb882cf99
```

### John the Ripper 模式

```powershell
.\haiti-lite-rust.exe --data-dir "C:\Tools\haiti\data" jtr 5f4dcc3b5aa765d61d8327deb882cf99
```

### 包含扩展规则

默认不显示标记为 extended 的规则，使用 `-e` 或 `--extended` 显示全部匹配：

```powershell
.\haiti-lite-rust.exe --data-dir ".\data" hc --extended 5f4dcc3b5aa765d61d8327deb882cf99
```

### 从标准输入读取

hash 参数使用 `-` 时，程序从标准输入读取：

```powershell
Get-Content .\hash.txt -Raw | .\haiti-lite-rust.exe --data-dir ".\data" hc -
```

### 查看版本和调试信息

```powershell
.\haiti-lite-rust.exe --version
.\haiti-lite-rust.exe --data-dir ".\data" hc --debug 5f4dcc3b5aa765d61d8327deb882cf99
```

## JSON 响应格式

### 识别成功

```json
{
  "mode": "hc",
  "hash": "5f4dcc3b5aa765d61d8327deb882cf99",
  "identified": true,
  "matches": [
    {
      "name": "MD5",
      "reference": 0
    },
    {
      "name": "NTLM",
      "reference": 1000
    }
  ]
}
```

`hc` 模式的 `reference` 是 JSON 数字，`jtr` 模式的 `reference` 是 JSON 字符串。`matches` 的顺序与规则数据的匹配顺序一致。

### 未识别

没有 prototype 匹配时程序仍然返回成功状态并输出：

```json
{
  "mode": "hc",
  "hash": "not-a-hash",
  "identified": false,
  "matches": []
}
```

### 调试响应

使用 `--debug` 时响应会增加 `debug` 字段：

```json
{
  "mode": "hc",
  "hash": "5f4dcc3b5aa765d61d8327deb882cf99",
  "identified": true,
  "matches": [
    {
      "name": "MD5",
      "reference": 0
    }
  ],
  "debug": {
    "data_dir": ".\\data",
    "extended": false
  }
}
```

### 版本响应

```json
{
  "name": "haiti-lite-rust",
  "version": "0.1.0"
}
```

### 错误响应

错误信息写入标准错误，格式也是 JSON：

```json
{
  "error": "could not read data\\prototypes.json"
}
```

## 脚本调用约定

- 标准输出始终是一条完整 JSON。
- `0` 表示调用成功，包括 `identified: false`。
- `2` 表示参数错误、规则目录错误、JSON 错误或规则编译错误。

PowerShell 示例：

```powershell
$exe = ".\haiti-lite-rust.exe"
$data = ".\data"
$hash = "5f4dcc3b5aa765d61d8327deb882cf99"

$raw = (& $exe --data-dir $data hc $hash | Out-String).Trim()
$exitCode = $LASTEXITCODE
if ($exitCode -ne 0) {
    throw "haiti-lite-rust failed with exit code $exitCode"
}

$response = $raw | ConvertFrom-Json
foreach ($match in $response.matches) {
    "$($match.name): $($match.reference)"
}
```
