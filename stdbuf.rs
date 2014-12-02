#![crate_name = "stdbuf"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Dorota Kapturkiewicz <dokaptur@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate libc;
use getopts::{optopt, optflag, getopts, usage, Matches, OptGroup};
use std::os;
use std::num;
use std::io::Command;
use std::iter::range_inclusive;
use std::num::Int;

static NAME: &'static str = "stdbuf";
static VERSION: &'static str = "1.0.0";

#[deriving(Show)]
enum BufferType {
	Default,
	Unbuffered,
	Line,
	Size(u64)
}

#[deriving(Show)]
struct ProgramOptions {
	stdin : BufferType,
	stdout : BufferType,
	stderr : BufferType,
}

enum ErrMsg {
	Retry,
	Fatal
}

enum OkMsg {
	Buffering,
	Help,
	Version
}

fn print_version() {
	println!("{} version {}", NAME, VERSION);
}

fn print_usage(opts: &[OptGroup]) {
	let brief = 
		"Usage: stdbuf OPTION... COMMAND\nRun COMMAND, with modified buffering operations for its standard streams\nMandatory arguments to long options are mandatory for short options too.";
	let explaination = 
		"If MODE is 'L' the corresponding stream will be line buffered.\nThis option is invalid with standard input.\n\nIf MODE is '0' the corresponding stream will be unbuffered.\n\nOtherwise MODE is a number which may be followed by one of the following:\n\nKB 1000, K 1024, MB 1000*1000, M 1024*1024, and so on for G, T, P, E, Z, Y.\nIn this case the corresponding stream will be fully buffered with the buffer size set to MODE bytes.\n\nNOTE: If COMMAND adjusts the buffering of its standard streams ('tee' does for e.g.) then that will override corresponding settings changed by 'stdbuf'.\nAlso some filters (like 'dd' and 'cat' etc.) don't use streams for I/O, and are thus unaffected by 'stdbuf' settings.\n";
	println!("{}\n{}", getopts::usage(brief, opts), explaination);
}

fn parse_size(size : &str) -> Option<u64> {
	let ext = size.trim_left_chars(|c: char| c.is_digit(10));
	let num = size.trim_right_chars(|c: char| c.is_alphabetic());
	let mut recovered = num.to_string();
	recovered.push_str(ext);
	if recovered.as_slice() != size {
		return None;
	}
	let buf_size : u64 = match from_str(num) {
		Some(m) => m,
		None => return None,
	};
	let (power, base) : (uint, u64) = match ext {
		"" => (0, 0),
		"KB" => (1, 1024),
		"K" => (1, 1000),
		"MB" => (2, 1024),
		"M" => (2, 1000),
		"GB" => (3, 1024),
		"G" => (3, 1000),
		"TB" => (4, 1024),
		"T" => (4, 1000),
		"PB" => (5, 1024),
		"P" => (5, 1000),
		"EB" => (6, 1024),
		"E" => (6, 1000),
		"ZB" => (7, 1024),
		"Z" => (7, 1000),
		"YB" => (8, 1024),
		"Y" => (8, 1000),
		_ => return None,
	};
	Some(buf_size * base.pow(power))
}

fn check_option(matches : &Matches, name : &str, modified : &mut bool) -> Option<BufferType> {
	match matches.opt_str(name) {
		Some(value) => {
			*modified = true;
			match value.as_slice() {
				"0" => Some(BufferType::Unbuffered),
				"L" => {
					if name == "input" {
						println!("stdbuf: line buffering stdin is meaningless");
						None
					} else {
						Some(BufferType::Line)
					}
				},
				x => {
					let size = match parse_size(x) {
						Some(m) => m,
						None => { println!("Invalid mode {}", x); return None }
					};
					Some(BufferType::Size(size))
				},
			}
		},
		None => Some(BufferType::Default),
	}
}

fn parse_options(args : &[String], options : &mut ProgramOptions, optgrps : &[OptGroup]) -> Result<OkMsg, ErrMsg> {
	let matches = match getopts(args, optgrps) {
		Ok(m) => m,
		Err(_) => return Err(ErrMsg::Retry)
	};
	if matches.opt_present("help") {
		return Ok(OkMsg::Help);
	}
	if matches.opt_present("version") {
		return Ok(OkMsg::Version);
	}
	let mut modified = false;
	options.stdin = try!(check_option(&matches, "input", &mut modified).ok_or(ErrMsg::Fatal));
	options.stdout = try!(check_option(&matches, "output", &mut modified).ok_or(ErrMsg::Fatal));
	options.stderr = try!(check_option(&matches, "error", &mut modified).ok_or(ErrMsg::Fatal));
	
	if matches.free.len() != 1 {
		return Err(ErrMsg::Retry);
	}
	if !modified {
		println!("stdbuf: you must specify a buffering mode option");
		return Err(ErrMsg::Fatal);
	}
	Ok(OkMsg::Buffering)
}


fn main() {
	let args = os::args();
	let optgrps = [
		optopt("i", "input", "adjust standard input stream buffering", "MODE"),
		optopt("o", "output", "adjust standard output stream buffering", "MODE"),
		optopt("e", "error", "adjust standard error stream buffering", "MODE"),
		optflag("", "help", "display this help and exit"),
		optflag("", "version", "output version information and exit"),
	];
	let mut options = ProgramOptions{ stdin : BufferType::Default, stdout : BufferType::Default, stderr : BufferType::Default};
	let mut command_idx = -1;
	for i in range_inclusive(1, args.len()) {
		match parse_options(args.slice(1, i), &mut options, &optgrps) {
			Ok(OkMsg::Buffering) => {
				command_idx = i-1;
				println!("Program arg index = {}", command_idx);
				break;
			},
			Ok(OkMsg::Help) => {
				print_usage(&optgrps);
				return;
			},
			Ok(OkMsg::Version) => {
				print_version();
				return;
			},
			Err(ErrMsg::Fatal) => break,
			Err(ErrMsg::Retry) => continue,
		}
	};
	if command_idx == -1 {
		println!("Invalid options\nTry 'stdbuf --help' for more information.");
		std::os::set_exit_status(125);
		return;
	}
	println!("{}", options);

	let ref command_name = args[command_idx];
	let mut process = match Command::new(command_name).args(args.slice_from(command_idx+1)).spawn() {
		Ok(p) => p,
		Err(e) => panic!("failed to execute process: {}", e),
	};
	
	let output = process.stdout.as_mut().unwrap().read_to_string().ok().expect("failed to read output");
	println!("{}", output);
}

