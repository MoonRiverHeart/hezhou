import { add, multiply, getMessage } from 'libentry.so';

@Entry
@Component
struct Index {
  @State message: string = '点击按钮调用 Rust';
  @State calcResult: string = '';

  build() {
    Column() {
      Text(this.message)
        .fontSize(24)
        .fontWeight(FontWeight.Bold)
        .margin({ top: 20, bottom: 10 })

      Text(this.calcResult)
        .fontSize(18)
        .margin({ bottom: 20 })

      Button('调用 Rust getMessage()')
        .onClick(() => {
          this.message = getMessage();
        })
        .margin({ bottom: 10 })

      Button('计算 10 + 20 (Rust)')
        .onClick(() => {
          this.calcResult = `10 + 20 = ${add(10, 20)}`;
        })
        .margin({ bottom: 10 })

      Button('计算 5 × 6 (Rust)')
        .onClick(() => {
          this.calcResult = `5 × 6 = ${multiply(5, 6)}`;
        })
    }
    .width('100%')
    .height('100%')
    .justifyContent(FlexAlign.Center)
  }
}
