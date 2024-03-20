# Distributor

## Overview 概述

**Distributor** 是一个由 Rust 开发的文件分发器，提供了基于配置文件的资产分发功能。

其提供了一个愚蠢的功能，为某些愚蠢的项目管理工具服务。

v0.1.0
by LviatYi

## Usage 使用

Distributor 提供了一组 Cli 命令，用于运行分发与配置管理。

配置文件默认存储在 `./distributor.toml` ，toml 是一种人类易读的 (human-editable) 文件类型，因此你也可以手动配置它。

请运行 `distributor.exe -h` 查看帮助。

```shell
./distributor.exe -h
```

## Example

将 `/resource/` 目录下的文件分发到 `/run/` 目录下。

```shell
./distributor.exe add -s resource/ -t run/
```

输出配置。

```shell
./distributor.exe list
```

## License 许可

This project is licensed under the [MIT license](./LICENSE.txt).
