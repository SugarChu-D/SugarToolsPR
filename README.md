気が向いたら追記します。

## GPU tuning

`config.toml` のトップレベルに `workgroup_size` を置くと、GPU ごとに compute workgroup size を指定できます。未指定時は `256` です。

```toml
workgroup_size = 64

[ds_configs.profile1]
version = "White2"
region = "JPN"
timer0 = 0x10fa
is_dslite = false
mac = 0x0009bf6d93ce
```

まずは `64`、`128`、`256` を比較してください。値は実行時に shader pipeline へ適用されるため、値ごとに別の実行ファイルをビルドする必要はありません。

## リリースの仕方

`cargo build -p [アプリ名] --release`でreleaseフォルダにexeファイルが生成される。
