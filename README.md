気が向いたら追記します。

## GPU tuning

`ds_config.json` のトップレベルに `workgroup_size` を置くと、GPU ごとに compute workgroup size を指定できます。未指定時は `256` です。

```json
{
  "workgroup_size": 128,
  "ds_configs": {
    "profile1": { "...": "既存のDS設定" }
  }
}
```

まずは `64`、`128`、`256` を比較してください。値は実行時に shader pipeline へ適用されるため、値ごとに別の実行ファイルをビルドする必要はありません。
