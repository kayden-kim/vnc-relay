//! `vnc-relay` 실행파일의 진입점이다.
//!
//! 하위 명령(`serve`, `admin`)은 아직 없다. 지금은 CLI 계약 1.1절의 `--version`만 처리한다.

use std::process::ExitCode;

/// CLI 계약 2.1절의 사용법 오류 종료 코드다.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    match (args.next(), args.next()) {
        (Some(flag), None) if flag == "--version" || flag == "-V" => {
            println!("vnc-relay {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("error[usage_error]: 지원하는 명령이 아직 없다. `--version`만 쓸 수 있다.");
            ExitCode::from(EXIT_USAGE)
        }
    }
}
