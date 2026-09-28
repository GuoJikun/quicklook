# QuickLook Privacy Policy

**Last Updated: September 28, 2026**

QuickLook is an open-source Windows application that provides quick previews of files and documents.

This Privacy Policy explains what information QuickLook processes, what information may be transmitted outside your device, and how third-party components used by QuickLook may handle data.

## 1. Overview

QuickLook is designed to process files locally on your Windows device.

QuickLook does **not intentionally upload the original files you preview to a server operated by QuickLook**.

However, QuickLook uses third-party components, including Microsoft Edge WebView2 and Sentry for diagnostics and application monitoring. These components may process or transmit certain technical information according to their own privacy mechanisms and policies.

Therefore, QuickLook should not be described as an application that guarantees that **no information ever leaves your device**.

---

## 2. File Contents

QuickLook may access and process the contents of files that you choose to preview.

Depending on the file type, QuickLook may read:

* Text content
* Images
* PDF documents
* Office documents
* Video and audio files
* Archive files
* Other supported document formats

File contents are processed locally on your device for the purpose of providing the preview functionality.

QuickLook does not intentionally upload the original contents of your files to a QuickLook-operated server.

Third-party components used by QuickLook may have their own data collection mechanisms. Please refer to the relevant sections of this policy for more information.

---

## 3. File Paths and File Names

QuickLook processes file paths and file names locally because they are required to locate and preview files.

File paths and file names may also appear in diagnostic information if they are included in an application error, exception, log, or other diagnostic event.

In particular, information associated with an error or performance event may be transmitted to Sentry when Sentry monitoring is enabled.

QuickLook does not intentionally collect file paths or file names for advertising, marketing, or user profiling purposes.

---

## 4. File Metadata

QuickLook may read file metadata required to display or process a preview.

Depending on the file type, this may include information such as:

* File size
* File modification time
* File type
* Image dimensions
* Media duration
* Document properties
* Other metadata required by the preview implementation

This information is primarily processed locally.

If metadata is included in an error or diagnostic event, some of it may be transmitted to Sentry as part of that diagnostic event.

---

## 5. Sentry Error Monitoring

QuickLook uses **Sentry** for production error monitoring.

Sentry may receive technical information associated with application errors and exceptions, which may include:

* Application version
* Operating system information
* Device or runtime information
* Error messages
* Stack traces
* Application state associated with an error
* Performance-related information
* Other technical diagnostic information

Depending on the error and the information available at the time, diagnostic events may also contain contextual information such as file paths, file names, or other values involved in the operation that produced the error.

QuickLook does not intentionally attach or upload the original file being previewed as part of a Sentry error report.

Sentry is a third-party service. Information processed by Sentry is subject to Sentry's own privacy practices and applicable service configuration.

---

## 6. Performance Monitoring and Tracing

QuickLook may use Sentry performance monitoring and tracing to understand application performance and diagnose performance-related problems.

This may include technical information such as:

* Operation duration
* Application transactions
* Performance measurements
* Runtime information
* Error context
* Application version
* Operating system information

Performance monitoring is used for software development, troubleshooting, and improving application reliability.

QuickLook does not use this information for advertising or behavioral profiling.

---

## 7. Session Replay

**QuickLook does not use Sentry Session Replay.**

Session Replay is disabled, and QuickLook does not intentionally generate or transmit session replay recordings.

---

## 8. Telemetry

QuickLook does not operate a separate advertising, marketing, or behavioral tracking system.

QuickLook may transmit technical diagnostic and performance information through Sentry when production monitoring is enabled.

This information is used to:

* Detect application errors
* Diagnose application failures
* Monitor application performance
* Improve application stability
* Investigate technical problems

QuickLook does not intentionally use diagnostic information to build advertising profiles or track users across unrelated applications or services.

---

## 9. Diagnostics and Crash Information

QuickLook may generate diagnostic information when an application error occurs.

Diagnostic information may include technical details such as:

* Error messages
* Stack traces
* Application version
* Operating system version
* Runtime information
* Performance information
* Application state
* Context associated with the failed operation

Diagnostic information may unintentionally contain file paths, file names, or other user-provided values if those values are present in the relevant application state or error message.

QuickLook does not intentionally upload complete documents or original file contents as crash-report attachments.

---

## 10. Personal Information

QuickLook does not require users to create an account.

QuickLook does not intentionally collect information such as:

* Name
* Email address
* Telephone number
* Postal address
* Account credentials

However, technical diagnostic services such as Sentry may process identifiers or other technical information according to their own service configuration and privacy policies.

---

## 11. Network Communications

QuickLook's core file preview functionality is designed to operate locally.

Network communication may occur for purposes such as:

* Error reporting
* Performance monitoring
* Diagnostic services
* Third-party runtime functionality
* Software or runtime updates, where applicable

QuickLook does not intentionally upload the original files being previewed to a QuickLook-operated server.

---

## 12. Microsoft Edge WebView2

QuickLook uses **Microsoft Edge WebView2** as part of its Windows application architecture.

WebView2 is a Microsoft component and has its own diagnostic and privacy behavior.

Microsoft documents that WebView2 can collect required diagnostic data and may collect optional diagnostic data. WebView2 can use Microsoft diagnostic infrastructure, including Chromium/Microsoft Edge telemetry infrastructure, Windows data reporting, and Microsoft crash-reporting infrastructure.

The exact diagnostic data collected by WebView2 can depend on Microsoft, Windows, WebView2, and device privacy settings.

QuickLook does not control all data collection performed by WebView2 or Windows.

For more information, please refer to Microsoft's documentation:

* [Data and privacy in WebView2](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy)
* [User data and privacy in Microsoft Edge](https://learn.microsoft.com/en-us/microsoft-edge/privacy-whitepaper/)

---

## 13. Windows Diagnostic Data

Windows itself may collect diagnostic data independently of QuickLook.

Microsoft provides different Windows diagnostic data settings, including required and optional diagnostic data. Optional diagnostic data can include additional information about device configuration, application activity, performance, and enhanced error reporting.

QuickLook cannot control diagnostic data collected directly by the Windows operating system.

Users can review and configure applicable Windows diagnostic settings under:

**Settings → Privacy & security → Diagnostics & feedback**

The available settings and behavior may vary depending on the Windows version, device configuration, organization policies, and Microsoft services.

---

## 14. Third-Party Components

QuickLook relies on third-party libraries, runtimes, and services.

These components may process technical information according to their own privacy policies and configurations.

Important third-party components include:

* Microsoft Edge WebView2
* Sentry
* Other open-source libraries included in the QuickLook software

Users should review the privacy policies and documentation of these third-party services where appropriate.

---

## 15. Data Security

QuickLook is an open-source application and processes the majority of file preview data locally on the user's device.

When diagnostic information is transmitted to third-party services such as Sentry, transmission is performed through the mechanisms provided by those services.

No method of transmission or storage can be guaranteed to be completely secure.

Users should avoid opening sensitive files in environments where third-party diagnostic or runtime services are not acceptable.

---

## 16. Data Retention

QuickLook itself does not maintain a centralized database containing users' previewed files.

Information transmitted through third-party services such as Sentry is retained according to the applicable service configuration and retention policies.

QuickLook does not independently control the retention period of data stored by Microsoft, Sentry, or other third-party services.

---

## 17. Children's Privacy

QuickLook is a general-purpose desktop application and is not specifically directed at children.

QuickLook does not knowingly collect personal information from children for advertising or profiling purposes.

---

## 18. Changes to This Privacy Policy

This Privacy Policy may be updated when QuickLook's functionality, third-party services, or data-processing practices change.

The latest version will be published in this repository.

The **Last Updated** date at the beginning of this document indicates when this policy was most recently revised.

---

## 19. Contact

If you have questions or concerns about this Privacy Policy or QuickLook's privacy practices, please open an issue in the QuickLook repository:

<https://github.com/GuoJikun/quicklook/issues>

---

## 20. Source Code

QuickLook is open-source software.

The source code is available at:

<https://github.com/GuoJikun/quicklook>

Users can inspect the source code to understand how QuickLook processes files and communicates with third-party services.

---

# QuickLook 隐私政策

**最后更新：2026 年 9 月 28 日**

QuickLook 是一个开源的 Windows 文件快速预览应用程序。

本隐私政策用于说明 QuickLook 会处理哪些信息、哪些信息可能离开您的设备，以及 QuickLook 所使用的第三方组件可能如何处理相关数据。

## 1. 概述

QuickLook 设计为在您的 Windows 设备本地处理文件。

QuickLook **不会主动将您预览的原始文件上传到由 QuickLook 运营的服务器**。

但是，QuickLook 使用了包括 Microsoft Edge WebView2 和 Sentry 在内的第三方组件，用于应用程序运行、错误监控和性能诊断。这些组件可能根据其自身的隐私机制和政策处理或传输部分技术信息。

因此，QuickLook 不应被描述为能够保证**任何信息都不会离开用户设备**的应用程序。

---

## 2. 文件内容

QuickLook 可能会访问和处理您选择进行预览的文件内容。

根据文件类型不同，QuickLook 可能读取：

* 文本内容
* 图片
* PDF 文档
* Office 文档
* 视频和音频文件
* 压缩文件
* 其他受支持的文档格式

这些文件内容主要在您的设备本地进行处理，以实现文件预览功能。

QuickLook 不会主动将您文件的原始内容上传到由 QuickLook 运营的服务器。

但是，QuickLook 使用的第三方组件可能具有独立的数据收集机制，具体情况请参阅本政策中的相关章节。

---

## 3. 文件路径和文件名

QuickLook 需要在本地处理文件路径和文件名，以定位和预览文件。

如果文件路径、文件名等信息包含在应用程序错误、异常、日志或其他诊断事件中，这些信息可能成为诊断信息的一部分。

特别是在启用生产环境 Sentry 监控的情况下，与错误或性能事件相关的信息可能被发送到 Sentry。

QuickLook 不会主动为了广告、营销或用户画像而收集文件路径或文件名。

---

## 4. 文件元数据

QuickLook 可能读取实现文件预览所需的文件元数据。

根据文件类型不同，这些信息可能包括：

* 文件大小
* 文件修改时间
* 文件类型
* 图片尺寸
* 媒体时长
* 文档属性
* 预览实现所需的其他元数据

这些信息主要在本地进行处理。

如果某些元数据包含在错误或诊断事件中，则其中部分信息可能作为诊断事件的一部分发送至 Sentry。

---

## 5. Sentry 错误监控

QuickLook 在生产环境中使用 **Sentry** 进行错误监控。

Sentry 可能接收与应用程序错误和异常相关的技术信息，包括：

* 应用程序版本
* 操作系统信息
* 设备或运行时信息
* 错误消息
* 堆栈信息
* 与错误相关的应用程序状态
* 性能相关信息
* 其他技术诊断信息

根据发生错误时的具体情况，诊断事件还可能包含文件路径、文件名或导致错误的相关操作中涉及的其他信息。

QuickLook 不会主动将用户正在预览的完整原始文件作为 Sentry 错误报告的附件上传。

Sentry 是第三方服务。Sentry 所处理的信息受 Sentry 自身隐私政策以及相关服务配置约束。

---

## 6. 性能监控和追踪

QuickLook 可能使用 Sentry 的性能监控和追踪功能，以了解应用程序性能并诊断性能相关问题。

这些信息可能包括：

* 操作耗时
* 应用程序事务
* 性能指标
* 运行时信息
* 错误上下文
* 应用程序版本
* 操作系统信息

性能监控用于软件开发、问题排查以及提高应用程序稳定性。

QuickLook 不会将这些信息用于广告或用户行为画像。

---

## 7. Session Replay 会话回放

**QuickLook 不使用 Sentry Session Replay。**

Session Replay 已关闭，QuickLook 不会主动生成或传输会话回放记录。

---

## 8. 遥测

QuickLook 不运营独立的广告、营销或用户行为跟踪系统。

在生产环境监控启用的情况下，QuickLook 可能通过 Sentry 传输技术诊断和性能信息。

这些信息用于：

* 发现应用程序错误
* 诊断应用程序故障
* 监控应用程序性能
* 改进应用程序稳定性
* 调查技术问题

QuickLook 不会主动使用这些诊断信息建立广告用户画像，也不会用于跨其他应用程序或服务跟踪用户。

---

## 9. 诊断和崩溃信息

QuickLook 在发生应用程序错误时可能生成诊断信息。

诊断信息可能包括：

* 错误消息
* 堆栈信息
* 应用程序版本
* 操作系统版本
* 运行时信息
* 性能信息
* 应用程序状态
* 与发生错误的操作相关的上下文信息

如果文件路径、文件名或其他用户输入的数据存在于相关应用程序状态或错误信息中，诊断信息可能会无意中包含这些信息。

QuickLook 不会主动将完整文档或原始文件内容作为崩溃报告附件上传。

---

## 10. 个人信息

QuickLook 不要求用户创建账户。

QuickLook 不会主动要求或收集以下信息：

* 姓名
* 电子邮件地址
* 电话号码
* 邮政地址
* 账户密码

但是，Sentry 等技术诊断服务可能根据自身服务配置和隐私政策处理技术标识符或其他技术信息。

---

## 11. 网络通信

QuickLook 的核心文件预览功能设计为在本地运行。

网络通信可能用于：

* 错误报告
* 性能监控
* 诊断服务
* 第三方运行时功能
* 软件或运行时更新（如果适用）

QuickLook 不会主动将正在预览的原始文件上传到由 QuickLook 运营的服务器。

---

## 12. Microsoft Edge WebView2

QuickLook 使用 **Microsoft Edge WebView2** 作为 Windows 应用程序架构的一部分。

WebView2 是 Microsoft 提供的组件，并具有独立的诊断和隐私行为。

Microsoft 官方文档说明，WebView2 可以收集必要诊断数据，也可能收集可选诊断数据。WebView2 可以使用 Chromium/Microsoft Edge 遥测基础设施、Windows 数据报告以及 Microsoft 崩溃报告基础设施等机制。

WebView2 实际收集的数据可能取决于 Microsoft、Windows、WebView2 以及设备自身的隐私设置。

QuickLook 无法控制 WebView2 或 Windows 执行的全部数据收集行为。

更多信息请参考 Microsoft 官方文档：

* [WebView2 中的数据和隐私](https://learn.microsoft.com/zh-cn/microsoft-edge/webview2/concepts/data-privacy)
* [Microsoft Edge 用户数据和隐私](https://learn.microsoft.com/zh-cn/microsoft-edge/privacy-whitepaper/)

---

## 13. Windows 诊断数据

Windows 操作系统本身可能独立于 QuickLook 收集诊断数据。

Microsoft 提供不同级别的 Windows 诊断数据设置，包括必要诊断数据和可选诊断数据。可选诊断数据可能包含设备配置、应用程序活动、性能以及增强错误报告等信息。

QuickLook 无法控制 Windows 操作系统直接收集的诊断数据。

用户可以在以下位置查看和配置适用的 Windows 诊断设置：

**设置 → 隐私和安全性 → 诊断和反馈**

具体设置和行为可能因 Windows 版本、设备配置、组织策略以及 Microsoft 服务而有所不同。

---

## 14. 第三方组件

QuickLook 使用了第三方库、运行时和服务。

这些组件可能根据其自身隐私政策和服务配置处理技术信息。

重要的第三方组件包括：

* Microsoft Edge WebView2
* Sentry
* QuickLook 使用的其他开源库

如有需要，用户可以进一步查阅相关第三方服务的隐私政策和技术文档。

---

## 15. 数据安全

QuickLook 是开源软件，并且大多数文件预览数据都在用户设备本地进行处理。

当诊断信息通过 Sentry 等第三方服务传输时，将使用这些服务提供的传输机制。

任何数据传输或存储方式都无法保证绝对安全。

如果用户处理高度敏感的文件，并且不希望相关技术诊断信息通过第三方服务进行处理，应根据自身需求谨慎使用 QuickLook。

---

## 16. 数据保留

QuickLook 本身不会维护包含用户预览文件的集中式数据库。

通过 Sentry 等第三方服务传输的信息，将根据相关服务的配置和数据保留政策进行保存。

QuickLook 无法独立控制 Microsoft、Sentry 或其他第三方服务所保存数据的具体保留期限。

---

## 17. 儿童隐私

QuickLook 是通用桌面应用程序，并非专门面向儿童设计。

QuickLook 不会明知地为了广告或用户画像目的收集儿童个人信息。

---

## 18. 隐私政策变更

当 QuickLook 的功能、第三方服务或数据处理方式发生变化时，本隐私政策可能进行更新。

最新版本将发布在 QuickLook 项目仓库中。

文档开头的 **Last Updated / 最后更新** 日期表示本隐私政策最近一次修改的时间。

---

## 19. 联系方式

如果您对本隐私政策或 QuickLook 的隐私处理方式有任何问题或疑问，可以通过 QuickLook GitHub 仓库提交 Issue：

<https://github.com/GuoJikun/quicklook/issues>

---

## 20. 源代码

QuickLook 是开源软件。

源代码地址：

<https://github.com/GuoJikun/quicklook>

用户可以通过查看源代码了解 QuickLook 如何处理文件以及如何与第三方服务进行通信。
