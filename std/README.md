# BoxLang 标准库 (std)

这是 BoxLang 编程语言的标准库，提供了核心功能和平台支持。

## 目录结构

```
std/
├── core/              # 核心库 (no_std 环境可用)
│   ├── mod.box        # 核心库入口
│   ├── option.box     # Option<T> 类型
│   ├── result.box     # Result<T, E> 类型
│   ├── mem.box        # 内存管理
│   ├── slice.box      # 切片操作
│   └── str.box        # 字符串核心功能
├── std/               # 完整标准库
│   ├── mod.box        # 标准库入口
│   ├── io.box         # IO 操作
│   ├── math.box       # 数学函数
│   └── time.box       # 时间/日期
└── zetboxos/          # zetboxos 嵌入式专用
    ├── mod.box        # zetboxos 库入口
    ├── gpio.box       # GPIO 控制
    ├── uart.box       # 串口通信
    └── i2c.box        # I2C 通信
```

## 核心库 (core)

核心库提供了 BoxLang 的基础功能，可以在没有操作系统的环境下使用（no_std）。

### Option<T>

表示一个可能有值的类型：

```boxlang
use core::option::Option;

fn find_item(items: &[i32], target: i32) -> Option<usize> {
    for i in 0..items.len() {
        if items[i] == target {
            return Option::Some(i);
        }
    }
    Option::None
}

fn main() {
    let items = [1, 2, 3, 4, 5];
    
    match find_item(&items, 3) {
        Option::Some(index) => println("Found at index: {}", index),
        Option::None => println("Not found"),
    }
    
    // 使用 unwrap_or 提供默认值
    let result = find_item(&items, 10).unwrap_or(999);
}
```

### Result<T, E>

表示可能失败的操作结果：

```boxlang
use core::result::Result;

fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Result::Err("Division by zero".to_string())
    } else {
        Result::Ok(a / b)
    }
}

fn main() {
    match divide(10.0, 2.0) {
        Result::Ok(result) => println("Result: {}", result),
        Result::Err(error) => println("Error: {}", error),
    }
}
```

### 内存管理 (mem)

```boxlang
use core::mem::Box;

fn main() {
    // 在堆上分配值
    let value = Box::new(42);
    println("Value: {}", *value);  // 自动解引用
    
    // Box 离开作用域时自动释放内存
}
```

### 切片操作 (slice)

```boxlang
use core::slice;

fn main() {
    let arr = [1, 2, 3, 4, 5];
    let s = &arr[1..4];  // 切片
    
    println("Length: {}", slice::len(s));
    println("First: {}", slice::first(s).unwrap());
    
    // 查找元素
    if let Option::Some(index) = slice::find(s, &3) {
        println("Found 3 at index: {}", index);
    }
}
```

## 标准库 (std)

标准库提供了完整的操作系统支持功能。

### IO 操作 (io)

```boxlang
use std::io;

fn main() {
    // 标准输出
    io::println("Hello, World!");
    io::print("No newline");
    
    // 标准错误
    io::eprintln("Error message");
    
    // 格式化输出
    io::printfln("Hello, {}! You have {} messages.", &["Alice", "5"]);
    
    // 读取输入
    io::print("Enter your name: ");
    let name = io::read_line().unwrap();
    io::println("Hello, {}!", name);
}
```

### 数学函数 (math)

```boxlang
use std::math;

fn main() {
    // 基本运算
    let x = math::sqrt(16.0);      // 4.0
    let y = math::pow(2.0, 3.0);   // 8.0
    let z = math::abs(-5.0);       // 5.0
    
    // 三角函数
    let angle = math::PI / 4.0;
    let sin_val = math::sin(angle);
    let cos_val = math::cos(angle);
    
    // 对数
    let log_val = math::ln(math::E);  // 1.0
    
    // 随机数
    math::srand(12345);  // 设置种子
    let random_val = math::rand_range_i32(1, 100);
}
```

### 时间操作 (time)

```boxlang
use std::time;

fn main() {
    // 延时
    time::sleep_secs(1);
    time::sleep_millis(500);
    
    // Duration
    let duration = time::Duration::from_secs(5);
    time::sleep(duration);
    
    // 测量时间
    let start = time::Instant::now();
    // ... 执行一些操作
    let elapsed = start.elapsed();
    println("Elapsed: {}", time::format_duration(elapsed));
    
    // 计时器
    let timer = time::Timer::new(time::Duration::from_secs(1));
    loop {
        if timer.is_ready() {
            println("Timer tick!");
            timer.reset();
        }
    }
}
```

## zetboxos 嵌入式库 (zetboxos)

zetboxos 库提供了嵌入式系统的硬件控制功能。

### GPIO 控制 (gpio)

```boxlang
use zetboxos::gpio;

fn main() {
    // 初始化 GPIO 引脚
    let led = gpio::Pin::new(13, gpio::PinMode::Output).unwrap();
    
    // 控制 LED
    led.set_high().unwrap();
    time::sleep_millis(500);
    led.set_low().unwrap();
    
    // 使用 LED 结构
    let led2 = gpio::Led::new(14).unwrap();
    led2.blink(3, 200).unwrap();  // 闪烁 3 次，每次 200ms
    
    // 读取按钮
    let button = gpio::Button::new(0, gpio::PullMode::Up).unwrap();
    if button.is_pressed().unwrap() {
        println("Button pressed!");
    }
}
```

### UART 串口通信 (uart)

```boxlang
use zetboxos::uart;

fn main() {
    // 初始化 UART
    let config = uart::UartConfig::default();
    let serial = uart::Uart::new(uart::UartPort::Uart0, config).unwrap();
    
    // 发送数据
    serial.send_string("Hello, UART!\n").unwrap();
    serial.send_bytes(&[0x01, 0x02, 0x03]).unwrap();
    
    // 接收数据
    let mut buf = [0u8; 64];
    let received = serial.receive_bytes(&mut buf).unwrap();
    
    // 便捷函数
    uart::send_str("Hello\n").unwrap();
    if let Option::Some(byte) = uart::receive() {
        println("Received: {}", byte);
    }
}
```

### I2C 通信 (i2c)

```boxlang
use zetboxos::i2c;

fn main() {
    // 初始化 I2C
    let i2c = i2c::I2cMaster::new(
        i2c::I2cBus::I2c0,
        i2c::I2cConfig::fast_mode()
    ).unwrap();
    
    // 设备地址
    let addr = i2c::I2cAddress::new_7bit(0x50).unwrap();
    
    // 写入数据
    i2c.write(addr, &[0x00, 0x01, 0x02]).unwrap();
    
    // 读取数据
    let mut buf = [0u8; 4];
    i2c.read(addr, &mut buf).unwrap();
    
    // 寄存器操作
    i2c.write_reg(addr, 0x10, 0x55).unwrap();
    let value = i2c.read_reg(addr, 0x10).unwrap();
    
    // 扫描总线
    let devices = i2c.scan();
    for device in devices {
        println("Found device: 0x{:02X}", device);
    }
}
```

## 使用说明

### 导入模块

```boxlang
// 导入整个模块
use std::io;

// 导入特定项
use core::option::Option;
use core::option::Some;
use core::option::None;

// 使用预导入模块（推荐）
use std::prelude::*;
```

### 编译选项

- **Host 目标**（Windows/Linux/macOS）：使用完整标准库
- **zetboxos 目标**（嵌入式）：可以使用 core 和 zetboxos 模块

### 注意事项

1. `core` 模块不依赖操作系统，可以在任何环境下使用
2. `std` 模块需要操作系统支持
3. `zetboxos` 模块只在 zetboxos 嵌入式系统上可用
4. 某些功能使用 `extern "builtin"` 声明，需要编译器支持

## 贡献

欢迎为 BoxLang 标准库做出贡献！请遵循以下准则：

1. 保持代码简洁清晰
2. 添加适当的文档注释
3. 为公共 API 编写测试
4. 遵循现有的代码风格

## 许可证

BoxLang 标准库与 BoxLang 编译器使用相同的许可证。
