//! Native namespace implementations for File, Dir, Path, Env, Args, Shell, Http,
//! Json, Toml, Yaml, Base64, Url, DateTime, Duration, Time, Regex, Gzip, Zip,
//! Hash, Uuid, Random, Crypto, Gui

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::{Duration as StdDuration, Instant};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use chrono::{DateTime as ChronoDateTime, Datelike, Local, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use hex;
use hmac::{Hmac, Mac};
use md5::Md5;
use pbkdf2::pbkdf2_hmac_array;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use rand::Rng;
use regex::{Regex, RegexBuilder};
use serde_json;
use sha2::{Digest, Sha256, Sha512};
use uuid::Uuid;

use crate::bytecode::{
    FutureState, HashableValue, ImageWrapper, TcpListenerWrapper, TcpStreamWrapper,
    UdpSocketWrapper, Value, WeakRefValue, WebSocketServerConnWrapper, WebSocketServerWrapper,
    WebSocketWrapper, XmlDocumentWrapper,
};
use crate::data::{
    read_csv_with_options, read_json, read_parquet, sql_query, write_csv, write_json,
    write_parquet, AggOp, AggSpec, CubeBuilder, DataFrame, JoinSpec, Series, SqlContext,
};
use image::{imageops::FilterType, DynamicImage, ImageFormat};
use std::sync::Arc;
use sxd_document::parser as xml_parser;
use sxd_xpath::{evaluate_xpath, Context, Factory, Value as XPathValue};

/// Result type for native namespace methods
pub type NativeResult = Result<Value, String>;

// ============================================================================
// File Module
// ============================================================================

pub fn file_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "read_text" => file_read_text(args),
        "read_bytes" => file_read_bytes(args),
        "read_lines" => file_read_lines(args),
        "write_text" => file_write_text(args),
        "write_bytes" => file_write_bytes(args),
        "append" => file_append(args),
        "exists" => file_exists(args),
        "size" => file_size(args),
        "delete" | "remove" => file_delete(args),
        "copy" => file_copy(args),
        "rename" | "move" => file_rename(args),
        _ => Err(format!("File has no method '{method}'")),
    }
}

fn file_read_text(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "File.read_text() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    fs::read_to_string(&path)
        .map(Value::string)
        .map_err(|e| format!("failed to read file '{}': {}", path, e))
}

fn file_read_bytes(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "File.read_bytes() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    let bytes = fs::read(&path).map_err(|e| format!("failed to read file '{}': {}", path, e))?;
    let values: Vec<Value> = bytes.into_iter().map(|b| Value::Int(b as i64)).collect();
    Ok(Value::list(values))
}

fn file_read_lines(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "File.read_lines() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    let content =
        fs::read_to_string(&path).map_err(|e| format!("failed to read file '{}': {}", path, e))?;
    let lines: Vec<Value> = content.lines().map(Value::string).collect();
    Ok(Value::list(lines))
}

fn file_write_text(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "File.write_text() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    let content = get_string_arg(&args[1], "content")?;
    fs::write(&path, &content).map_err(|e| format!("failed to write file '{}': {}", path, e))?;
    Ok(Value::Null)
}

fn file_write_bytes(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "File.write_bytes() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    let bytes = get_bytes_arg(&args[1])?;
    fs::write(&path, &bytes).map_err(|e| format!("failed to write file '{}': {}", path, e))?;
    Ok(Value::Null)
}

fn file_append(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "File.append() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    let content = get_string_arg(&args[1], "content")?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("failed to open file '{}': {}", path, e))?;

    file.write_all(content.as_bytes())
        .map_err(|e| format!("failed to append to file '{}': {}", path, e))?;

    Ok(Value::Null)
}

fn file_exists(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "File.exists() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    Ok(Value::Bool(Path::new(&path).is_file()))
}

fn file_size(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "File.size() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    let metadata =
        fs::metadata(&path).map_err(|e| format!("failed to get metadata for '{}': {}", path, e))?;
    Ok(Value::Int(metadata.len() as i64))
}

fn file_delete(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "File.delete() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    fs::remove_file(&path).map_err(|e| format!("failed to delete file '{}': {}", path, e))?;
    Ok(Value::Null)
}

fn file_copy(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "File.copy() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let src = get_string_arg(&args[0], "source")?;
    let dst = get_string_arg(&args[1], "destination")?;
    let bytes_copied = fs::copy(&src, &dst)
        .map_err(|e| format!("failed to copy '{}' to '{}': {}", src, dst, e))?;
    Ok(Value::Int(bytes_copied as i64))
}

fn file_rename(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "File.rename() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let src = get_string_arg(&args[0], "source")?;
    let dst = get_string_arg(&args[1], "destination")?;
    fs::rename(&src, &dst)
        .map_err(|e| format!("failed to rename '{}' to '{}': {}", src, dst, e))?;
    Ok(Value::Null)
}

// ============================================================================
// Dir Module
// ============================================================================

pub fn dir_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "list" => dir_list(args),
        "create" => dir_create(args),
        "create_all" => dir_create_all(args),
        "remove" | "delete" => dir_remove(args),
        "remove_all" | "delete_all" => dir_remove_all(args),
        "exists" => dir_exists(args),
        _ => Err(format!("Dir has no method '{method}'")),
    }
}

fn dir_list(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Dir.list() expects 1 argument, got {}", args.len()));
    }
    let path = get_string_arg(&args[0], "path")?;
    let entries =
        fs::read_dir(&path).map_err(|e| format!("failed to read directory '{}': {}", path, e))?;

    let mut files: Vec<Value> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read entry: {}", e))?;
        if let Some(name) = entry.file_name().to_str() {
            files.push(Value::string(name));
        }
    }
    Ok(Value::list(files))
}

fn dir_create(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Dir.create() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    fs::create_dir(&path).map_err(|e| format!("failed to create directory '{}': {}", path, e))?;
    Ok(Value::Null)
}

fn dir_create_all(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Dir.create_all() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    fs::create_dir_all(&path)
        .map_err(|e| format!("failed to create directories '{}': {}", path, e))?;
    Ok(Value::Null)
}

fn dir_remove(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Dir.remove() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    fs::remove_dir(&path).map_err(|e| format!("failed to remove directory '{}': {}", path, e))?;
    Ok(Value::Null)
}

fn dir_remove_all(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Dir.remove_all() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    fs::remove_dir_all(&path)
        .map_err(|e| format!("failed to remove directory '{}': {}", path, e))?;
    Ok(Value::Null)
}

fn dir_exists(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Dir.exists() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    Ok(Value::Bool(Path::new(&path).is_dir()))
}

// ============================================================================
// Path Module
// ============================================================================

pub fn path_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "join" => path_join(args),
        "extension" | "ext" => path_extension(args),
        "filename" | "file_name" => path_filename(args),
        "parent" => path_parent(args),
        "stem" | "file_stem" => path_stem(args),
        "is_absolute" => path_is_absolute(args),
        "is_relative" => path_is_relative(args),
        "normalize" | "canonicalize" => path_normalize(args),
        "exists" => path_exists(args),
        "is_file" => path_is_file(args),
        "is_dir" => path_is_dir(args),
        _ => Err(format!("Path has no method '{method}'")),
    }
}

fn path_join(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("Path.join() expects at least 1 argument".to_string());
    }
    let mut path = std::path::PathBuf::new();
    for arg in args {
        let part = get_string_arg(arg, "path part")?;
        path.push(&part);
    }
    Ok(Value::string(path.to_string_lossy()))
}

fn path_extension(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.extension() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    match Path::new(&path).extension() {
        Some(ext) => Ok(Value::string(ext.to_string_lossy())),
        None => Ok(Value::Null),
    }
}

fn path_filename(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.filename() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    match Path::new(&path).file_name() {
        Some(name) => Ok(Value::string(name.to_string_lossy())),
        None => Ok(Value::Null),
    }
}

fn path_parent(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.parent() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    match Path::new(&path).parent() {
        Some(parent) => Ok(Value::string(parent.to_string_lossy())),
        None => Ok(Value::Null),
    }
}

fn path_stem(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.stem() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    match Path::new(&path).file_stem() {
        Some(stem) => Ok(Value::string(stem.to_string_lossy())),
        None => Ok(Value::Null),
    }
}

fn path_is_absolute(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.is_absolute() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    Ok(Value::Bool(Path::new(&path).is_absolute()))
}

fn path_is_relative(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.is_relative() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    Ok(Value::Bool(Path::new(&path).is_relative()))
}

fn path_normalize(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.normalize() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    match fs::canonicalize(&path) {
        Ok(canonical) => Ok(Value::string(canonical.to_string_lossy())),
        Err(e) => Err(format!("failed to normalize path '{}': {}", path, e)),
    }
}

fn path_exists(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.exists() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    Ok(Value::Bool(Path::new(&path).exists()))
}

fn path_is_file(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.is_file() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    Ok(Value::Bool(Path::new(&path).is_file()))
}

fn path_is_dir(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Path.is_dir() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    Ok(Value::Bool(Path::new(&path).is_dir()))
}

// ============================================================================
// Env Module
// ============================================================================

pub fn env_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "get" => env_get(args),
        "set" => env_set(args),
        "remove" | "unset" => env_remove(args),
        "all" | "vars" => env_all(args),
        "has" | "contains" => env_has(args),
        _ => Err(format!("Env has no method '{method}'")),
    }
}

fn env_get(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Env.get() expects 1-2 arguments, got {}",
            args.len()
        ));
    }
    let name = get_string_arg(&args[0], "name")?;
    match env::var(&name) {
        Ok(value) => Ok(Value::string(value)),
        Err(_) => {
            if args.len() == 2 {
                Ok(args[1].clone())
            } else {
                Ok(Value::Null)
            }
        }
    }
}

fn env_set(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!("Env.set() expects 2 arguments, got {}", args.len()));
    }
    let name = get_string_arg(&args[0], "name")?;
    let value = get_string_arg(&args[1], "value")?;
    env::set_var(&name, &value);
    Ok(Value::Null)
}

fn env_remove(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Env.remove() expects 1 argument, got {}",
            args.len()
        ));
    }
    let name = get_string_arg(&args[0], "name")?;
    env::remove_var(&name);
    Ok(Value::Null)
}

fn env_all(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!("Env.all() expects 0 arguments, got {}", args.len()));
    }
    let mut map = HashMap::new();
    for (key, value) in env::vars() {
        let k = HashableValue::String(Rc::new(key));
        let v = Value::string(value);
        map.insert(k, v);
    }
    Ok(Value::Map(Rc::new(std::cell::RefCell::new(map))))
}

fn env_has(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Env.has() expects 1 argument, got {}", args.len()));
    }
    let name = get_string_arg(&args[0], "name")?;
    Ok(Value::Bool(env::var(&name).is_ok()))
}

// ============================================================================
// Args Module
// ============================================================================

pub fn args_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "all" | "list" => args_all(args),
        "get" => args_get(args),
        "count" | "len" => args_count(args),
        _ => Err(format!("Args has no method '{method}'")),
    }
}

fn args_all(_args: &[Value]) -> NativeResult {
    let args: Vec<Value> = env::args().map(Value::string).collect();
    Ok(Value::list(args))
}

fn args_get(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Args.get() expects 1 argument, got {}", args.len()));
    }
    let index = match &args[0] {
        Value::Int(i) => *i as usize,
        _ => {
            return Err(format!(
                "Args.get() expects Int index, got {}",
                args[0].type_name()
            ))
        }
    };
    let cli_args: Vec<String> = env::args().collect();
    if index < cli_args.len() {
        Ok(Value::string(&cli_args[index]))
    } else {
        Ok(Value::Null)
    }
}

fn args_count(_args: &[Value]) -> NativeResult {
    Ok(Value::Int(env::args().count() as i64))
}

// ============================================================================
// Shell Module
// ============================================================================

pub fn shell_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "run" => shell_run(args),
        "exec" => shell_exec(args),
        _ => Err(format!("Shell has no method '{method}'")),
    }
}

fn shell_run(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Shell.run() expects 1-2 arguments, got {}",
            args.len()
        ));
    }
    let program = get_string_arg(&args[0], "program")?;
    let cmd_args: Vec<String> = if args.len() == 2 {
        match &args[1] {
            Value::List(list) => list
                .borrow()
                .iter()
                .map(|v| match v {
                    Value::String(s) => Ok(s.to_string()),
                    _ => Err(format!(
                        "Shell.run() argument must be string, got {}",
                        v.type_name()
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => {
                return Err(format!(
                    "Shell.run() expects List as second argument, got {}",
                    args[1].type_name()
                ))
            }
        }
    } else {
        Vec::new()
    };

    let output = Command::new(&program)
        .args(&cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to execute '{}': {}", program, e))?;

    // Create result map with stdout, stderr, exit_code
    let mut result = HashMap::new();
    result.insert(
        HashableValue::String(Rc::new("stdout".to_string())),
        Value::string(String::from_utf8_lossy(&output.stdout)),
    );
    result.insert(
        HashableValue::String(Rc::new("stderr".to_string())),
        Value::string(String::from_utf8_lossy(&output.stderr)),
    );
    result.insert(
        HashableValue::String(Rc::new("exit_code".to_string())),
        Value::Int(output.status.code().unwrap_or(-1) as i64),
    );
    result.insert(
        HashableValue::String(Rc::new("success".to_string())),
        Value::Bool(output.status.success()),
    );

    Ok(Value::Map(Rc::new(std::cell::RefCell::new(result))))
}

fn shell_exec(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Shell.exec() expects 1 argument, got {}",
            args.len()
        ));
    }
    let command = get_string_arg(&args[0], "command")?;

    // Use shell to execute the command
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/C", &command])
        .output()
        .map_err(|e| format!("failed to execute command: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .args(["-c", &command])
        .output()
        .map_err(|e| format!("failed to execute command: {}", e))?;

    if output.status.success() {
        Ok(Value::string(
            String::from_utf8_lossy(&output.stdout).trim_end(),
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "command failed with exit code {}: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ))
    }
}

// ============================================================================
// Http Module
// ============================================================================

pub fn http_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "get" => http_get(args),
        "post" => http_post(args),
        "put" => http_put(args),
        "patch" => http_patch(args),
        "delete" => http_delete(args),
        "head" => http_head(args),
        _ => Err(format!("Http has no method '{method}'")),
    }
}

/// Build a reqwest blocking client with optional timeout
fn build_http_client(timeout_ms: Option<i64>) -> Result<reqwest::blocking::Client, String> {
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(ms) = timeout_ms {
        builder = builder.timeout(StdDuration::from_millis(ms as u64));
    }
    builder
        .build()
        .map_err(|e| format!("failed to build HTTP client: {}", e))
}

/// Extract options from a Value::Map (headers, timeout)
fn extract_http_options(options: &Value) -> Result<(HashMap<String, String>, Option<i64>), String> {
    let mut headers = HashMap::new();
    let mut timeout = None;

    match options {
        Value::Map(map) => {
            let map = map.borrow();

            // Extract headers
            let headers_key = HashableValue::String(Rc::new("headers".to_string()));
            if let Some(Value::Map(h)) = map.get(&headers_key) {
                for (k, v) in h.borrow().iter() {
                    if let (HashableValue::String(key), Value::String(val)) = (k, v) {
                        headers.insert(key.to_string(), val.to_string());
                    }
                }
            }

            // Extract timeout
            let timeout_key = HashableValue::String(Rc::new("timeout".to_string()));
            if let Some(Value::Int(ms)) = map.get(&timeout_key) {
                timeout = Some(*ms);
            }
        }
        _ => return Err(format!("options must be Map, got {}", options.type_name())),
    }

    Ok((headers, timeout))
}

/// Convert a reqwest Response to a Stratum Value (Map with status, body, headers, ok)
fn response_to_value(response: reqwest::blocking::Response) -> NativeResult {
    let status = response.status().as_u16() as i64;
    let ok = response.status().is_success();

    // Collect response headers
    let mut resp_headers = HashMap::new();
    for (name, value) in response.headers().iter() {
        if let Ok(v) = value.to_str() {
            resp_headers.insert(
                HashableValue::String(Rc::new(name.to_string())),
                Value::string(v),
            );
        }
    }

    // Get body text
    let body = response
        .text()
        .map_err(|e| format!("failed to read response body: {}", e))?;

    // Build result map
    let mut result = HashMap::new();
    result.insert(
        HashableValue::String(Rc::new("status".to_string())),
        Value::Int(status),
    );
    result.insert(
        HashableValue::String(Rc::new("body".to_string())),
        Value::string(body),
    );
    result.insert(
        HashableValue::String(Rc::new("headers".to_string())),
        Value::Map(Rc::new(RefCell::new(resp_headers))),
    );
    result.insert(
        HashableValue::String(Rc::new("ok".to_string())),
        Value::Bool(ok),
    );

    Ok(Value::Map(Rc::new(RefCell::new(result))))
}

fn http_get(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Http.get() expects 1-2 arguments, got {}",
            args.len()
        ));
    }

    let url = get_string_arg(&args[0], "url")?;
    let (headers, timeout) = if args.len() == 2 {
        extract_http_options(&args[1])?
    } else {
        (HashMap::new(), None)
    };

    let client = build_http_client(timeout)?;
    let mut request = client.get(&url);

    for (name, value) in headers {
        request = request.header(&name, &value);
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP GET request failed: {}", e))?;

    response_to_value(response)
}

fn http_post(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "Http.post() expects 1-3 arguments, got {}",
            args.len()
        ));
    }

    let url = get_string_arg(&args[0], "url")?;
    let body = if args.len() >= 2 {
        get_string_arg(&args[1], "body")?
    } else {
        String::new()
    };
    let (headers, timeout) = if args.len() == 3 {
        extract_http_options(&args[2])?
    } else {
        (HashMap::new(), None)
    };

    let client = build_http_client(timeout)?;
    let mut request = client.post(&url).body(body);

    for (name, value) in headers {
        request = request.header(&name, &value);
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP POST request failed: {}", e))?;

    response_to_value(response)
}

fn http_put(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "Http.put() expects 1-3 arguments, got {}",
            args.len()
        ));
    }

    let url = get_string_arg(&args[0], "url")?;
    let body = if args.len() >= 2 {
        get_string_arg(&args[1], "body")?
    } else {
        String::new()
    };
    let (headers, timeout) = if args.len() == 3 {
        extract_http_options(&args[2])?
    } else {
        (HashMap::new(), None)
    };

    let client = build_http_client(timeout)?;
    let mut request = client.put(&url).body(body);

    for (name, value) in headers {
        request = request.header(&name, &value);
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP PUT request failed: {}", e))?;

    response_to_value(response)
}

fn http_patch(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "Http.patch() expects 1-3 arguments, got {}",
            args.len()
        ));
    }

    let url = get_string_arg(&args[0], "url")?;
    let body = if args.len() >= 2 {
        get_string_arg(&args[1], "body")?
    } else {
        String::new()
    };
    let (headers, timeout) = if args.len() == 3 {
        extract_http_options(&args[2])?
    } else {
        (HashMap::new(), None)
    };

    let client = build_http_client(timeout)?;
    let mut request = client.patch(&url).body(body);

    for (name, value) in headers {
        request = request.header(&name, &value);
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP PATCH request failed: {}", e))?;

    response_to_value(response)
}

fn http_delete(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Http.delete() expects 1-2 arguments, got {}",
            args.len()
        ));
    }

    let url = get_string_arg(&args[0], "url")?;
    let (headers, timeout) = if args.len() == 2 {
        extract_http_options(&args[1])?
    } else {
        (HashMap::new(), None)
    };

    let client = build_http_client(timeout)?;
    let mut request = client.delete(&url);

    for (name, value) in headers {
        request = request.header(&name, &value);
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP DELETE request failed: {}", e))?;

    response_to_value(response)
}

fn http_head(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Http.head() expects 1-2 arguments, got {}",
            args.len()
        ));
    }

    let url = get_string_arg(&args[0], "url")?;
    let (headers, timeout) = if args.len() == 2 {
        extract_http_options(&args[1])?
    } else {
        (HashMap::new(), None)
    };

    let client = build_http_client(timeout)?;
    let mut request = client.head(&url);

    for (name, value) in headers {
        request = request.header(&name, &value);
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP HEAD request failed: {}", e))?;

    // For HEAD requests, there's no body
    let status = response.status().as_u16() as i64;
    let ok = response.status().is_success();

    let mut resp_headers = HashMap::new();
    for (name, value) in response.headers().iter() {
        if let Ok(v) = value.to_str() {
            resp_headers.insert(
                HashableValue::String(Rc::new(name.to_string())),
                Value::string(v),
            );
        }
    }

    let mut result = HashMap::new();
    result.insert(
        HashableValue::String(Rc::new("status".to_string())),
        Value::Int(status),
    );
    result.insert(
        HashableValue::String(Rc::new("body".to_string())),
        Value::string(""),
    );
    result.insert(
        HashableValue::String(Rc::new("headers".to_string())),
        Value::Map(Rc::new(RefCell::new(resp_headers))),
    );
    result.insert(
        HashableValue::String(Rc::new("ok".to_string())),
        Value::Bool(ok),
    );

    Ok(Value::Map(Rc::new(RefCell::new(result))))
}

// ============================================================================
// Json Module
// ============================================================================

pub fn json_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "encode" | "stringify" => json_encode(args),
        "decode" | "parse" => json_decode(args),
        _ => Err(format!("Json has no method '{method}'")),
    }
}

fn json_encode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Json.encode() expects 1 argument, got {}",
            args.len()
        ));
    }
    let json_value = value_to_json(&args[0])?;
    let json_str =
        serde_json::to_string(&json_value).map_err(|e| format!("failed to encode JSON: {}", e))?;
    Ok(Value::string(json_str))
}

fn json_decode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Json.decode() expects 1 argument, got {}",
            args.len()
        ));
    }
    let json_str = get_string_arg(&args[0], "json")?;
    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("failed to parse JSON: {}", e))?;
    json_to_value(&json_value)
}

/// Convert a Stratum Value to a serde_json::Value
fn value_to_json(value: &Value) -> Result<serde_json::Value, String> {
    match value {
        Value::Null => Ok(serde_json::Value::Null),
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Int(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .ok_or_else(|| "cannot represent float in JSON (NaN or Infinity)".to_string()),
        Value::String(s) => Ok(serde_json::Value::String(s.to_string())),
        Value::List(list) => {
            let items: Result<Vec<_>, _> = list.borrow().iter().map(value_to_json).collect();
            Ok(serde_json::Value::Array(items?))
        }
        Value::Map(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map.borrow().iter() {
                let key = match k {
                    HashableValue::String(s) => s.to_string(),
                    HashableValue::Int(i) => i.to_string(),
                    HashableValue::Bool(b) => b.to_string(),
                    HashableValue::Null => "null".to_string(),
                };
                obj.insert(key, value_to_json(v)?);
            }
            Ok(serde_json::Value::Object(obj))
        }
        Value::Struct(s) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in s.borrow().fields.iter() {
                obj.insert(k.clone(), value_to_json(v)?);
            }
            Ok(serde_json::Value::Object(obj))
        }
        other => Err(format!("cannot convert {} to JSON", other.type_name())),
    }
}

/// Convert a serde_json::Value to a Stratum Value
fn json_to_value(json: &serde_json::Value) -> NativeResult {
    match json {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float(f))
            } else {
                Err("invalid JSON number".to_string())
            }
        }
        serde_json::Value::String(s) => Ok(Value::string(s.clone())),
        serde_json::Value::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.iter().map(json_to_value).collect();
            Ok(Value::list(items?))
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj.iter() {
                let key = HashableValue::String(Rc::new(k.clone()));
                map.insert(key, json_to_value(v)?);
            }
            Ok(Value::Map(Rc::new(RefCell::new(map))))
        }
    }
}

// ============================================================================
// Toml Module
// ============================================================================

pub fn toml_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "encode" | "stringify" => toml_encode(args),
        "decode" | "parse" => toml_decode(args),
        _ => Err(format!("Toml has no method '{method}'")),
    }
}

fn toml_encode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Toml.encode() expects 1 argument, got {}",
            args.len()
        ));
    }
    let toml_value = value_to_toml(&args[0])?;
    let toml_str =
        toml::to_string(&toml_value).map_err(|e| format!("failed to encode TOML: {}", e))?;
    Ok(Value::string(toml_str))
}

fn toml_decode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Toml.decode() expects 1 argument, got {}",
            args.len()
        ));
    }
    let toml_str = get_string_arg(&args[0], "toml")?;
    let toml_value: toml::Value =
        toml::from_str(&toml_str).map_err(|e| format!("failed to parse TOML: {}", e))?;
    toml_to_value(&toml_value)
}

/// Convert a Stratum Value to a toml::Value
fn value_to_toml(value: &Value) -> Result<toml::Value, String> {
    match value {
        Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        Value::Int(i) => Ok(toml::Value::Integer(*i)),
        Value::Float(f) => Ok(toml::Value::Float(*f)),
        Value::String(s) => Ok(toml::Value::String(s.to_string())),
        Value::List(list) => {
            let items: Result<Vec<_>, _> = list.borrow().iter().map(value_to_toml).collect();
            Ok(toml::Value::Array(items?))
        }
        Value::Map(map) => {
            let mut table = toml::map::Map::new();
            for (k, v) in map.borrow().iter() {
                let key = match k {
                    HashableValue::String(s) => s.to_string(),
                    _ => return Err("TOML keys must be strings".to_string()),
                };
                table.insert(key, value_to_toml(v)?);
            }
            Ok(toml::Value::Table(table))
        }
        Value::Struct(s) => {
            let mut table = toml::map::Map::new();
            for (k, v) in s.borrow().fields.iter() {
                table.insert(k.clone(), value_to_toml(v)?);
            }
            Ok(toml::Value::Table(table))
        }
        Value::Null => Err("TOML does not support null values".to_string()),
        other => Err(format!("cannot convert {} to TOML", other.type_name())),
    }
}

/// Convert a toml::Value to a Stratum Value
fn toml_to_value(toml: &toml::Value) -> NativeResult {
    match toml {
        toml::Value::Boolean(b) => Ok(Value::Bool(*b)),
        toml::Value::Integer(i) => Ok(Value::Int(*i)),
        toml::Value::Float(f) => Ok(Value::Float(*f)),
        toml::Value::String(s) => Ok(Value::string(s.clone())),
        toml::Value::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.iter().map(toml_to_value).collect();
            Ok(Value::list(items?))
        }
        toml::Value::Table(table) => {
            let mut map = HashMap::new();
            for (k, v) in table.iter() {
                let key = HashableValue::String(Rc::new(k.clone()));
                map.insert(key, toml_to_value(v)?);
            }
            Ok(Value::Map(Rc::new(RefCell::new(map))))
        }
        toml::Value::Datetime(dt) => Ok(Value::string(dt.to_string())),
    }
}

// ============================================================================
// Yaml Module
// ============================================================================

pub fn yaml_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "encode" | "stringify" => yaml_encode(args),
        "decode" | "parse" => yaml_decode(args),
        _ => Err(format!("Yaml has no method '{method}'")),
    }
}

fn yaml_encode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Yaml.encode() expects 1 argument, got {}",
            args.len()
        ));
    }
    let yaml_value = value_to_yaml(&args[0])?;
    let yaml_str =
        serde_yaml::to_string(&yaml_value).map_err(|e| format!("failed to encode YAML: {}", e))?;
    Ok(Value::string(yaml_str))
}

fn yaml_decode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Yaml.decode() expects 1 argument, got {}",
            args.len()
        ));
    }
    let yaml_str = get_string_arg(&args[0], "yaml")?;
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&yaml_str).map_err(|e| format!("failed to parse YAML: {}", e))?;
    yaml_to_value(&yaml_value)
}

/// Convert a Stratum Value to a serde_yaml::Value
fn value_to_yaml(value: &Value) -> Result<serde_yaml::Value, String> {
    match value {
        Value::Null => Ok(serde_yaml::Value::Null),
        Value::Bool(b) => Ok(serde_yaml::Value::Bool(*b)),
        Value::Int(i) => Ok(serde_yaml::Value::Number(serde_yaml::Number::from(*i))),
        Value::Float(f) => Ok(serde_yaml::Value::Number(serde_yaml::Number::from(*f))),
        Value::String(s) => Ok(serde_yaml::Value::String(s.to_string())),
        Value::List(list) => {
            let items: Result<Vec<_>, _> = list.borrow().iter().map(value_to_yaml).collect();
            Ok(serde_yaml::Value::Sequence(items?))
        }
        Value::Map(map) => {
            let mut mapping = serde_yaml::Mapping::new();
            for (k, v) in map.borrow().iter() {
                let key = match k {
                    HashableValue::String(s) => serde_yaml::Value::String(s.to_string()),
                    HashableValue::Int(i) => {
                        serde_yaml::Value::Number(serde_yaml::Number::from(*i))
                    }
                    HashableValue::Bool(b) => serde_yaml::Value::Bool(*b),
                    HashableValue::Null => serde_yaml::Value::Null,
                };
                mapping.insert(key, value_to_yaml(v)?);
            }
            Ok(serde_yaml::Value::Mapping(mapping))
        }
        Value::Struct(s) => {
            let mut mapping = serde_yaml::Mapping::new();
            for (k, v) in s.borrow().fields.iter() {
                mapping.insert(serde_yaml::Value::String(k.clone()), value_to_yaml(v)?);
            }
            Ok(serde_yaml::Value::Mapping(mapping))
        }
        other => Err(format!("cannot convert {} to YAML", other.type_name())),
    }
}

/// Convert a serde_yaml::Value to a Stratum Value
fn yaml_to_value(yaml: &serde_yaml::Value) -> NativeResult {
    match yaml {
        serde_yaml::Value::Null => Ok(Value::Null),
        serde_yaml::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float(f))
            } else {
                Err("invalid YAML number".to_string())
            }
        }
        serde_yaml::Value::String(s) => Ok(Value::string(s.clone())),
        serde_yaml::Value::Sequence(seq) => {
            let items: Result<Vec<_>, _> = seq.iter().map(yaml_to_value).collect();
            Ok(Value::list(items?))
        }
        serde_yaml::Value::Mapping(mapping) => {
            let mut map = HashMap::new();
            for (k, v) in mapping.iter() {
                let key = match k {
                    serde_yaml::Value::String(s) => HashableValue::String(Rc::new(s.clone())),
                    serde_yaml::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            HashableValue::Int(i)
                        } else {
                            return Err("YAML map keys must be strings or integers".to_string());
                        }
                    }
                    serde_yaml::Value::Bool(b) => HashableValue::Bool(*b),
                    serde_yaml::Value::Null => HashableValue::Null,
                    _ => return Err("YAML map keys must be scalar values".to_string()),
                };
                map.insert(key, yaml_to_value(v)?);
            }
            Ok(Value::Map(Rc::new(RefCell::new(map))))
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_value(&tagged.value),
    }
}

// ============================================================================
// Base64 Module
// ============================================================================

pub fn base64_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "encode" => base64_encode(args),
        "decode" => base64_decode(args),
        _ => Err(format!("Base64 has no method '{method}'")),
    }
}

fn base64_encode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Base64.encode() expects 1 argument, got {}",
            args.len()
        ));
    }

    let bytes = match &args[0] {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::List(_) => get_bytes_arg(&args[0])?,
        _ => {
            return Err(format!(
                "Base64.encode() expects String or List<Int>, got {}",
                args[0].type_name()
            ))
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(Value::string(encoded))
}

fn base64_decode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Base64.decode() expects 1 argument, got {}",
            args.len()
        ));
    }

    let encoded = get_string_arg(&args[0], "base64 string")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .map_err(|e| format!("failed to decode base64: {}", e))?;

    // Try to decode as UTF-8 string, otherwise return as byte list
    match String::from_utf8(bytes.clone()) {
        Ok(s) => Ok(Value::string(s)),
        Err(_) => {
            let values: Vec<Value> = bytes.into_iter().map(|b| Value::Int(b as i64)).collect();
            Ok(Value::list(values))
        }
    }
}

// ============================================================================
// Url Module
// ============================================================================

pub fn url_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "encode" => url_encode(args),
        "decode" => url_decode(args),
        _ => Err(format!("Url has no method '{method}'")),
    }
}

fn url_encode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Url.encode() expects 1 argument, got {}",
            args.len()
        ));
    }

    let input = get_string_arg(&args[0], "string")?;
    let encoded = utf8_percent_encode(&input, NON_ALPHANUMERIC).to_string();
    Ok(Value::string(encoded))
}

fn url_decode(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Url.decode() expects 1 argument, got {}",
            args.len()
        ));
    }

    let input = get_string_arg(&args[0], "encoded string")?;
    let decoded = percent_decode_str(&input)
        .decode_utf8()
        .map_err(|e| format!("failed to decode URL: {}", e))?
        .to_string();
    Ok(Value::string(decoded))
}

// ============================================================================
// DateTime Module
// ============================================================================

pub fn datetime_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "now" => datetime_now(args),
        "parse" => datetime_parse(args),
        "from_timestamp" => datetime_from_timestamp(args),
        "format" => datetime_format(args),
        "year" => datetime_component(args, "year"),
        "month" => datetime_component(args, "month"),
        "day" => datetime_component(args, "day"),
        "hour" => datetime_component(args, "hour"),
        "minute" => datetime_component(args, "minute"),
        "second" => datetime_component(args, "second"),
        "millisecond" => datetime_component(args, "millisecond"),
        "weekday" => datetime_weekday(args),
        "timestamp" => datetime_component(args, "timestamp"),
        "add" => datetime_add(args),
        "subtract" => datetime_subtract(args),
        "diff" => datetime_diff(args),
        "compare" => datetime_compare(args),
        "to_utc" => datetime_to_utc(args),
        "to_local" => datetime_to_local(args),
        "to_timezone" => datetime_to_timezone(args),
        _ => Err(format!("DateTime has no method '{method}'")),
    }
}

/// Create a datetime map from chrono DateTime
fn chrono_to_value<Tz: TimeZone>(dt: &ChronoDateTime<Tz>, tz_name: &str) -> Value {
    let mut map = HashMap::new();
    map.insert(
        HashableValue::String(Rc::new("year".to_string())),
        Value::Int(i64::from(dt.year())),
    );
    map.insert(
        HashableValue::String(Rc::new("month".to_string())),
        Value::Int(i64::from(dt.month())),
    );
    map.insert(
        HashableValue::String(Rc::new("day".to_string())),
        Value::Int(i64::from(dt.day())),
    );
    map.insert(
        HashableValue::String(Rc::new("hour".to_string())),
        Value::Int(i64::from(dt.hour())),
    );
    map.insert(
        HashableValue::String(Rc::new("minute".to_string())),
        Value::Int(i64::from(dt.minute())),
    );
    map.insert(
        HashableValue::String(Rc::new("second".to_string())),
        Value::Int(i64::from(dt.second())),
    );
    map.insert(
        HashableValue::String(Rc::new("millisecond".to_string())),
        Value::Int(i64::from(dt.timestamp_subsec_millis())),
    );
    map.insert(
        HashableValue::String(Rc::new("timestamp".to_string())),
        Value::Int(dt.timestamp_millis()),
    );
    map.insert(
        HashableValue::String(Rc::new("timezone".to_string())),
        Value::string(tz_name),
    );
    Value::Map(Rc::new(RefCell::new(map)))
}

/// Extract timestamp from a datetime map
fn get_datetime_timestamp(value: &Value) -> Result<i64, String> {
    match value {
        Value::Map(map) => {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new("timestamp".to_string()));
            match map.get(&key) {
                Some(Value::Int(ts)) => Ok(*ts),
                _ => Err("datetime must have 'timestamp' field".to_string()),
            }
        }
        _ => Err(format!("expected DateTime map, got {}", value.type_name())),
    }
}

/// Extract timezone from a datetime map
fn get_datetime_timezone(value: &Value) -> Result<String, String> {
    match value {
        Value::Map(map) => {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new("timezone".to_string()));
            match map.get(&key) {
                Some(Value::String(tz)) => Ok(tz.to_string()),
                _ => Ok("UTC".to_string()),
            }
        }
        _ => Err(format!("expected DateTime map, got {}", value.type_name())),
    }
}

fn datetime_now(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "DateTime.now() expects 0 arguments, got {}",
            args.len()
        ));
    }
    let now = Local::now();
    Ok(chrono_to_value(&now, "Local"))
}

fn datetime_parse(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "DateTime.parse() expects 1-2 arguments, got {}",
            args.len()
        ));
    }
    let input = get_string_arg(&args[0], "datetime string")?;

    // Try parsing with format if provided
    if args.len() == 2 {
        let format = get_string_arg(&args[1], "format")?;
        let naive = NaiveDateTime::parse_from_str(&input, &format).map_err(|e| {
            format!(
                "failed to parse datetime '{}' with format '{}': {}",
                input, format, e
            )
        })?;
        let dt = Utc.from_utc_datetime(&naive);
        return Ok(chrono_to_value(&dt, "UTC"));
    }

    // Try ISO 8601 / RFC 3339 format first
    if let Ok(dt) = ChronoDateTime::parse_from_rfc3339(&input) {
        return Ok(chrono_to_value(&dt.with_timezone(&Utc), "UTC"));
    }

    // Try RFC 2822
    if let Ok(dt) = ChronoDateTime::parse_from_rfc2822(&input) {
        return Ok(chrono_to_value(&dt.with_timezone(&Utc), "UTC"));
    }

    // Try common formats
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d",
        "%d-%m-%Y %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
    ];

    for fmt in formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&input, fmt) {
            let dt = Utc.from_utc_datetime(&naive);
            return Ok(chrono_to_value(&dt, "UTC"));
        }
        // Try date-only formats
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&input, fmt) {
            let naive = date.and_hms_opt(0, 0, 0).unwrap();
            let dt = Utc.from_utc_datetime(&naive);
            return Ok(chrono_to_value(&dt, "UTC"));
        }
    }

    Err(format!("failed to parse datetime: '{}'", input))
}

fn datetime_from_timestamp(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "DateTime.from_timestamp() expects 1 argument, got {}",
            args.len()
        ));
    }
    let millis = get_int_arg(&args[0], "timestamp")?;
    let dt = Utc
        .timestamp_millis_opt(millis)
        .single()
        .ok_or_else(|| format!("invalid timestamp: {}", millis))?;
    Ok(chrono_to_value(&dt, "UTC"))
}

fn datetime_format(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "DateTime.format() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let ts = get_datetime_timestamp(&args[0])?;
    let format = get_string_arg(&args[1], "format")?;

    let dt = Utc
        .timestamp_millis_opt(ts)
        .single()
        .ok_or_else(|| format!("invalid timestamp: {}", ts))?;

    Ok(Value::string(dt.format(&format).to_string()))
}

fn datetime_component(args: &[Value], component: &str) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "DateTime.{}() expects 1 argument, got {}",
            component,
            args.len()
        ));
    }

    match &args[0] {
        Value::Map(map) => {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new(component.to_string()));
            match map.get(&key) {
                Some(value) => Ok(value.clone()),
                None => Err(format!("datetime has no '{}' field", component)),
            }
        }
        _ => Err(format!(
            "expected DateTime map, got {}",
            args[0].type_name()
        )),
    }
}

fn datetime_weekday(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "DateTime.weekday() expects 1 argument, got {}",
            args.len()
        ));
    }
    let ts = get_datetime_timestamp(&args[0])?;
    let dt = Utc
        .timestamp_millis_opt(ts)
        .single()
        .ok_or_else(|| format!("invalid timestamp: {}", ts))?;

    let weekday = match dt.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    };
    Ok(Value::string(weekday))
}

fn datetime_add(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "DateTime.add() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let ts = get_datetime_timestamp(&args[0])?;
    let tz = get_datetime_timezone(&args[0])?;
    let duration_millis = get_duration_millis(&args[1])?;

    let new_ts = ts + duration_millis;
    let dt = Utc
        .timestamp_millis_opt(new_ts)
        .single()
        .ok_or_else(|| format!("invalid resulting timestamp: {}", new_ts))?;
    Ok(chrono_to_value(&dt, &tz))
}

fn datetime_subtract(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "DateTime.subtract() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let ts = get_datetime_timestamp(&args[0])?;
    let tz = get_datetime_timezone(&args[0])?;
    let duration_millis = get_duration_millis(&args[1])?;

    let new_ts = ts - duration_millis;
    let dt = Utc
        .timestamp_millis_opt(new_ts)
        .single()
        .ok_or_else(|| format!("invalid resulting timestamp: {}", new_ts))?;
    Ok(chrono_to_value(&dt, &tz))
}

fn datetime_diff(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "DateTime.diff() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let ts1 = get_datetime_timestamp(&args[0])?;
    let ts2 = get_datetime_timestamp(&args[1])?;

    let diff_millis = ts1 - ts2;
    Ok(duration_to_value(diff_millis))
}

fn datetime_compare(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "DateTime.compare() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let ts1 = get_datetime_timestamp(&args[0])?;
    let ts2 = get_datetime_timestamp(&args[1])?;

    let cmp = if ts1 < ts2 {
        -1
    } else if ts1 > ts2 {
        1
    } else {
        0
    };
    Ok(Value::Int(cmp))
}

fn datetime_to_utc(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "DateTime.to_utc() expects 1 argument, got {}",
            args.len()
        ));
    }
    let ts = get_datetime_timestamp(&args[0])?;
    let dt = Utc
        .timestamp_millis_opt(ts)
        .single()
        .ok_or_else(|| format!("invalid timestamp: {}", ts))?;
    Ok(chrono_to_value(&dt, "UTC"))
}

fn datetime_to_local(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "DateTime.to_local() expects 1 argument, got {}",
            args.len()
        ));
    }
    let ts = get_datetime_timestamp(&args[0])?;
    let dt = Local
        .timestamp_millis_opt(ts)
        .single()
        .ok_or_else(|| format!("invalid timestamp: {}", ts))?;
    Ok(chrono_to_value(&dt, "Local"))
}

fn datetime_to_timezone(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "DateTime.to_timezone() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let ts = get_datetime_timestamp(&args[0])?;
    let tz_name = get_string_arg(&args[1], "timezone")?;

    let tz: Tz = tz_name
        .parse()
        .map_err(|_| format!("invalid timezone: '{}'", tz_name))?;

    let dt_utc = Utc
        .timestamp_millis_opt(ts)
        .single()
        .ok_or_else(|| format!("invalid timestamp: {}", ts))?;
    let dt = dt_utc.with_timezone(&tz);

    Ok(chrono_to_value(&dt, &tz_name))
}

// ============================================================================
// Duration Module
// ============================================================================

pub fn duration_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "milliseconds" | "millis" => duration_milliseconds(args),
        "seconds" | "secs" => duration_seconds(args),
        "minutes" | "mins" => duration_minutes(args),
        "hours" => duration_hours(args),
        "days" => duration_days(args),
        "as_millis" => duration_as_millis(args),
        "as_secs" => duration_as_secs(args),
        "as_mins" => duration_as_mins(args),
        "as_hours" => duration_as_hours(args),
        "as_days" => duration_as_days(args),
        "add" => duration_add(args),
        "subtract" => duration_subtract(args),
        _ => Err(format!("Duration has no method '{method}'")),
    }
}

/// Create a duration value (map with millis field)
fn duration_to_value(millis: i64) -> Value {
    let mut map = HashMap::new();
    map.insert(
        HashableValue::String(Rc::new("millis".to_string())),
        Value::Int(millis),
    );
    Value::Map(Rc::new(RefCell::new(map)))
}

/// Extract milliseconds from a duration map
fn get_duration_millis(value: &Value) -> Result<i64, String> {
    match value {
        Value::Map(map) => {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new("millis".to_string()));
            match map.get(&key) {
                Some(Value::Int(ms)) => Ok(*ms),
                _ => Err("duration must have 'millis' field".to_string()),
            }
        }
        Value::Int(ms) => Ok(*ms), // Allow raw int as millis
        _ => Err(format!("expected Duration map, got {}", value.type_name())),
    }
}

fn duration_milliseconds(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.milliseconds() expects 1 argument, got {}",
            args.len()
        ));
    }
    let ms = get_int_arg(&args[0], "milliseconds")?;
    Ok(duration_to_value(ms))
}

fn duration_seconds(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.seconds() expects 1 argument, got {}",
            args.len()
        ));
    }
    let secs = get_int_arg(&args[0], "seconds")?;
    Ok(duration_to_value(secs * 1000))
}

fn duration_minutes(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.minutes() expects 1 argument, got {}",
            args.len()
        ));
    }
    let mins = get_int_arg(&args[0], "minutes")?;
    Ok(duration_to_value(mins * 60 * 1000))
}

fn duration_hours(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.hours() expects 1 argument, got {}",
            args.len()
        ));
    }
    let hours = get_int_arg(&args[0], "hours")?;
    Ok(duration_to_value(hours * 60 * 60 * 1000))
}

fn duration_days(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.days() expects 1 argument, got {}",
            args.len()
        ));
    }
    let days = get_int_arg(&args[0], "days")?;
    Ok(duration_to_value(days * 24 * 60 * 60 * 1000))
}

fn duration_as_millis(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.as_millis() expects 1 argument, got {}",
            args.len()
        ));
    }
    let millis = get_duration_millis(&args[0])?;
    Ok(Value::Int(millis))
}

fn duration_as_secs(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.as_secs() expects 1 argument, got {}",
            args.len()
        ));
    }
    let millis = get_duration_millis(&args[0])?;
    Ok(Value::Float(millis as f64 / 1000.0))
}

fn duration_as_mins(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.as_mins() expects 1 argument, got {}",
            args.len()
        ));
    }
    let millis = get_duration_millis(&args[0])?;
    Ok(Value::Float(millis as f64 / (60.0 * 1000.0)))
}

fn duration_as_hours(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.as_hours() expects 1 argument, got {}",
            args.len()
        ));
    }
    let millis = get_duration_millis(&args[0])?;
    Ok(Value::Float(millis as f64 / (60.0 * 60.0 * 1000.0)))
}

fn duration_as_days(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Duration.as_days() expects 1 argument, got {}",
            args.len()
        ));
    }
    let millis = get_duration_millis(&args[0])?;
    Ok(Value::Float(millis as f64 / (24.0 * 60.0 * 60.0 * 1000.0)))
}

fn duration_add(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Duration.add() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let ms1 = get_duration_millis(&args[0])?;
    let ms2 = get_duration_millis(&args[1])?;
    Ok(duration_to_value(ms1 + ms2))
}

fn duration_subtract(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Duration.subtract() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let ms1 = get_duration_millis(&args[0])?;
    let ms2 = get_duration_millis(&args[1])?;
    Ok(duration_to_value(ms1 - ms2))
}

// ============================================================================
// Time Module
// ============================================================================

pub fn time_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "sleep" => time_sleep(args),
        "sleep_ms" => time_sleep_ms(args),
        "start" => time_start(args),
        "elapsed" => time_elapsed(args),
        _ => Err(format!("Time has no method '{method}'")),
    }
}

/// Global start time for elapsed time calculations
/// We use Instant to measure elapsed time, but store as millis since program start
fn get_instant_millis() -> i64 {
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    start.elapsed().as_millis() as i64
}

fn time_sleep(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Time.sleep() expects 1 argument, got {}",
            args.len()
        ));
    }
    let millis = get_duration_millis(&args[0])?;
    if millis < 0 {
        return Err("sleep duration cannot be negative".to_string());
    }
    std::thread::sleep(StdDuration::from_millis(millis as u64));
    Ok(Value::Null)
}

fn time_sleep_ms(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Time.sleep_ms() expects 1 argument, got {}",
            args.len()
        ));
    }
    let ms = get_int_arg(&args[0], "milliseconds")?;
    if ms < 0 {
        return Err("sleep duration cannot be negative".to_string());
    }
    std::thread::sleep(StdDuration::from_millis(ms as u64));
    Ok(Value::Null)
}

fn time_start(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "Time.start() expects 0 arguments, got {}",
            args.len()
        ));
    }
    // Return a timer value with the current instant millis
    let mut map = HashMap::new();
    map.insert(
        HashableValue::String(Rc::new("_start_millis".to_string())),
        Value::Int(get_instant_millis()),
    );
    Ok(Value::Map(Rc::new(RefCell::new(map))))
}

fn time_elapsed(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Time.elapsed() expects 1 argument, got {}",
            args.len()
        ));
    }

    let start_millis = match &args[0] {
        Value::Map(map) => {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new("_start_millis".to_string()));
            match map.get(&key) {
                Some(Value::Int(ms)) => *ms,
                _ => return Err("timer must have '_start_millis' field".to_string()),
            }
        }
        _ => return Err(format!("expected timer map, got {}", args[0].type_name())),
    };

    let elapsed = get_instant_millis() - start_millis;
    Ok(duration_to_value(elapsed))
}

// ============================================================================
// Regex Module
// ============================================================================

pub fn regex_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "new" => regex_new(args),
        "is_match" => regex_is_match(args),
        "find" => regex_find(args),
        "find_all" => regex_find_all(args),
        "replace" => regex_replace(args),
        "replace_all" => regex_replace_all(args),
        "split" => regex_split(args),
        "captures" => regex_captures(args),
        _ => Err(format!("Regex has no method '{method}'")),
    }
}

/// Regex.new(pattern) or Regex.new(pattern, options)
/// Returns a compiled Regex value
fn regex_new(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Regex.new() expects 1-2 arguments, got {}",
            args.len()
        ));
    }

    let pattern = get_string_arg(&args[0], "pattern")?;
    let options = args.get(1);

    let re = build_regex(&pattern, options)?;
    Ok(Value::regex(re))
}

/// Build a regex from pattern string and optional options map
fn build_regex(pattern: &str, options: Option<&Value>) -> Result<Regex, String> {
    let mut builder = RegexBuilder::new(pattern);

    if let Some(opts) = options {
        if let Value::Map(map) = opts {
            let map = map.borrow();

            // Check for case_insensitive option
            let ci_key = HashableValue::String(Rc::new("case_insensitive".to_string()));
            if let Some(Value::Bool(true)) = map.get(&ci_key) {
                builder.case_insensitive(true);
            }

            // Check for multiline option
            let ml_key = HashableValue::String(Rc::new("multiline".to_string()));
            if let Some(Value::Bool(true)) = map.get(&ml_key) {
                builder.multi_line(true);
            }

            // Check for dot_matches_newline option
            let dot_key = HashableValue::String(Rc::new("dot_matches_newline".to_string()));
            if let Some(Value::Bool(true)) = map.get(&dot_key) {
                builder.dot_matches_new_line(true);
            }
        }
    }

    builder
        .build()
        .map_err(|e| format!("invalid regex pattern: {}", e))
}

/// Get a regex from the first argument - either a compiled Regex value or a pattern string
/// Returns (Regex, index of next argument after regex/pattern+options)
fn get_regex_arg(args: &[Value]) -> Result<(Rc<Regex>, usize), String> {
    if args.is_empty() {
        return Err("expected regex pattern or compiled Regex".to_string());
    }

    match &args[0] {
        // If first arg is already a compiled Regex, use it directly
        Value::Regex(re) => Ok((Rc::clone(re), 1)),

        // If first arg is a string, compile it (with optional options map as second arg)
        Value::String(pattern) => {
            let options = args.get(1).filter(|v| matches!(v, Value::Map(_)));
            let re = build_regex(pattern, options)?;
            let next_idx = if options.is_some() { 2 } else { 1 };
            Ok((Rc::new(re), next_idx))
        }

        _ => Err(format!(
            "expected regex pattern (String) or compiled Regex, got {}",
            args[0].type_name()
        )),
    }
}

/// Create a match result map from a regex match
fn match_to_value(m: &regex::Match, text: &str) -> Value {
    let mut map = HashMap::new();

    // The matched text
    map.insert(
        HashableValue::String(Rc::new("text".to_string())),
        Value::string(m.as_str()),
    );

    // Start position
    map.insert(
        HashableValue::String(Rc::new("start".to_string())),
        Value::Int(m.start() as i64),
    );

    // End position
    map.insert(
        HashableValue::String(Rc::new("end".to_string())),
        Value::Int(m.end() as i64),
    );

    // Also include the full input text for context
    let _ = text; // Silence unused warning - kept for potential future use

    Value::Map(Rc::new(RefCell::new(map)))
}

/// Regex.is_match(regex, text) or Regex.is_match(pattern, text) or Regex.is_match(pattern, options, text)
fn regex_is_match(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "Regex.is_match() expects at least 2 arguments, got {}",
            args.len()
        ));
    }

    let (re, next_idx) = get_regex_arg(args)?;

    if next_idx >= args.len() {
        return Err("Regex.is_match() requires a text argument".to_string());
    }

    let text = get_string_arg(&args[next_idx], "text")?;
    Ok(Value::Bool(re.is_match(&text)))
}

/// Regex.find(regex, text) or Regex.find(pattern, text) or Regex.find(pattern, options, text)
/// Returns the first match as a map, or null if no match
fn regex_find(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "Regex.find() expects at least 2 arguments, got {}",
            args.len()
        ));
    }

    let (re, next_idx) = get_regex_arg(args)?;

    if next_idx >= args.len() {
        return Err("Regex.find() requires a text argument".to_string());
    }

    let text = get_string_arg(&args[next_idx], "text")?;

    match re.find(&text) {
        Some(m) => Ok(match_to_value(&m, &text)),
        None => Ok(Value::Null),
    }
}

/// Regex.find_all(regex, text) or Regex.find_all(pattern, text) or Regex.find_all(pattern, options, text)
/// Returns a list of all matches
fn regex_find_all(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "Regex.find_all() expects at least 2 arguments, got {}",
            args.len()
        ));
    }

    let (re, next_idx) = get_regex_arg(args)?;

    if next_idx >= args.len() {
        return Err("Regex.find_all() requires a text argument".to_string());
    }

    let text = get_string_arg(&args[next_idx], "text")?;

    let matches: Vec<Value> = re
        .find_iter(&text)
        .map(|m| match_to_value(&m, &text))
        .collect();

    Ok(Value::list(matches))
}

/// Regex.replace(regex, text, replacement) or Regex.replace(pattern, text, replacement)
/// Replaces the first match
fn regex_replace(args: &[Value]) -> NativeResult {
    if args.len() < 3 {
        return Err(format!(
            "Regex.replace() expects at least 3 arguments, got {}",
            args.len()
        ));
    }

    let (re, next_idx) = get_regex_arg(args)?;

    if next_idx + 1 >= args.len() {
        return Err("Regex.replace() requires text and replacement arguments".to_string());
    }

    let text = get_string_arg(&args[next_idx], "text")?;
    let replacement = get_string_arg(&args[next_idx + 1], "replacement")?;

    let result = re.replace(&text, replacement.as_str());

    Ok(Value::string(&*result))
}

/// Regex.replace_all(regex, text, replacement) or Regex.replace_all(pattern, text, replacement)
/// Replaces all matches
fn regex_replace_all(args: &[Value]) -> NativeResult {
    if args.len() < 3 {
        return Err(format!(
            "Regex.replace_all() expects at least 3 arguments, got {}",
            args.len()
        ));
    }

    let (re, next_idx) = get_regex_arg(args)?;

    if next_idx + 1 >= args.len() {
        return Err("Regex.replace_all() requires text and replacement arguments".to_string());
    }

    let text = get_string_arg(&args[next_idx], "text")?;
    let replacement = get_string_arg(&args[next_idx + 1], "replacement")?;

    let result = re.replace_all(&text, replacement.as_str());

    Ok(Value::string(&*result))
}

/// Regex.split(regex, text) or Regex.split(pattern, text)
/// Splits text by the pattern
fn regex_split(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "Regex.split() expects at least 2 arguments, got {}",
            args.len()
        ));
    }

    let (re, next_idx) = get_regex_arg(args)?;

    if next_idx >= args.len() {
        return Err("Regex.split() requires a text argument".to_string());
    }

    let text = get_string_arg(&args[next_idx], "text")?;
    let parts: Vec<Value> = re.split(&text).map(Value::string).collect();

    Ok(Value::list(parts))
}

/// Regex.captures(regex, text) or Regex.captures(pattern, text)
/// Returns capture groups from the first match as a list, or null if no match
/// The first element is the full match, followed by each capture group
fn regex_captures(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "Regex.captures() expects at least 2 arguments, got {}",
            args.len()
        ));
    }

    let (re, next_idx) = get_regex_arg(args)?;

    if next_idx >= args.len() {
        return Err("Regex.captures() requires a text argument".to_string());
    }

    let text = get_string_arg(&args[next_idx], "text")?;

    match re.captures(&text) {
        Some(caps) => {
            let mut result = Vec::new();

            // Iterate over all capture groups (including group 0 = full match)
            for i in 0..caps.len() {
                match caps.get(i) {
                    Some(m) => result.push(Value::string(m.as_str())),
                    None => result.push(Value::Null),
                }
            }

            Ok(Value::list(result))
        }
        None => Ok(Value::Null),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_int_arg(value: &Value, name: &str) -> Result<i64, String> {
    match value {
        Value::Int(i) => Ok(*i),
        _ => Err(format!("{} must be Int, got {}", name, value.type_name())),
    }
}

fn get_string_arg(value: &Value, name: &str) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.to_string()),
        _ => Err(format!(
            "{} must be String, got {}",
            name,
            value.type_name()
        )),
    }
}

fn get_bytes_arg(value: &Value) -> Result<Vec<u8>, String> {
    match value {
        Value::List(list) => list
            .borrow()
            .iter()
            .map(|v| match v {
                Value::Int(i) if *i >= 0 && *i <= 255 => Ok(*i as u8),
                Value::Int(i) => Err(format!("byte value {} out of range 0-255", i)),
                _ => Err(format!("bytes must be Int, got {}", v.type_name())),
            })
            .collect(),
        _ => Err(format!("bytes must be List, got {}", value.type_name())),
    }
}

// ============================================================================
// Gzip Module
// ============================================================================

/// Gzip module entry point - compression and decompression
pub fn gzip_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "compress" => gzip_compress(args),
        "decompress" => gzip_decompress(args),
        "compress_text" => gzip_compress_text(args),
        "decompress_text" => gzip_decompress_text(args),
        _ => Err(format!("Gzip has no method '{method}'")),
    }
}

/// Gzip.compress(bytes: List<Int>) -> List<Int>
/// Compresses bytes using gzip
fn gzip_compress(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Gzip.compress() expects 1 argument, got {}",
            args.len()
        ));
    }
    let bytes = get_bytes_arg(&args[0])?;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&bytes)
        .map_err(|e| format!("gzip compression failed: {}", e))?;
    let compressed = encoder
        .finish()
        .map_err(|e| format!("gzip compression failed: {}", e))?;

    Ok(bytes_to_list(&compressed))
}

/// Gzip.decompress(bytes: List<Int>) -> List<Int>
/// Decompresses gzip-encoded bytes
fn gzip_decompress(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Gzip.decompress() expects 1 argument, got {}",
            args.len()
        ));
    }
    let bytes = get_bytes_arg(&args[0])?;

    let mut decoder = GzDecoder::new(&bytes[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("gzip decompression failed: {}", e))?;

    Ok(bytes_to_list(&decompressed))
}

/// Gzip.compress_text(text: String) -> List<Int>
/// Compresses a string using gzip
fn gzip_compress_text(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Gzip.compress_text() expects 1 argument, got {}",
            args.len()
        ));
    }
    let text = get_string_arg(&args[0], "text")?;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(text.as_bytes())
        .map_err(|e| format!("gzip compression failed: {}", e))?;
    let compressed = encoder
        .finish()
        .map_err(|e| format!("gzip compression failed: {}", e))?;

    Ok(bytes_to_list(&compressed))
}

/// Gzip.decompress_text(bytes: List<Int>) -> String
/// Decompresses gzip-encoded bytes to a string
fn gzip_decompress_text(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Gzip.decompress_text() expects 1 argument, got {}",
            args.len()
        ));
    }
    let bytes = get_bytes_arg(&args[0])?;

    let mut decoder = GzDecoder::new(&bytes[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("gzip decompression failed: {}", e))?;

    String::from_utf8(decompressed)
        .map(Value::string)
        .map_err(|e| format!("decompressed data is not valid UTF-8: {}", e))
}

/// Helper to convert bytes to Value::List
fn bytes_to_list(bytes: &[u8]) -> Value {
    let values: Vec<Value> = bytes.iter().map(|b| Value::Int(i64::from(*b))).collect();
    Value::list(values)
}

// ============================================================================
// Zip Module
// ============================================================================

/// Zip module entry point - zip archive operations
pub fn zip_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "list" => zip_list(args),
        "extract" => zip_extract(args),
        "extract_file" => zip_extract_file(args),
        "create" => zip_create(args),
        "read_text" => zip_read_text(args),
        "read_bytes" => zip_read_bytes(args),
        _ => Err(format!("Zip has no method '{method}'")),
    }
}

/// Zip.list(path: String) -> List<Map>
/// Lists all entries in a zip archive
fn zip_list(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Zip.list() expects 1 argument, got {}", args.len()));
    }
    let path = get_string_arg(&args[0], "path")?;

    let file =
        File::open(&path).map_err(|e| format!("failed to open zip file '{}': {}", path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("failed to read zip archive '{}': {}", path, e))?;

    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("failed to read entry {}: {}", i, e))?;

        let mut map = HashMap::new();
        map.insert(
            HashableValue::String(Rc::new("name".to_string())),
            Value::string(entry.name()),
        );
        map.insert(
            HashableValue::String(Rc::new("size".to_string())),
            Value::Int(entry.size() as i64),
        );
        map.insert(
            HashableValue::String(Rc::new("compressed_size".to_string())),
            Value::Int(entry.compressed_size() as i64),
        );
        map.insert(
            HashableValue::String(Rc::new("is_dir".to_string())),
            Value::Bool(entry.is_dir()),
        );

        entries.push(Value::Map(Rc::new(RefCell::new(map))));
    }

    Ok(Value::list(entries))
}

/// Zip.extract(path: String, output_dir: String) -> nil
/// Extracts all entries from a zip archive to a directory
fn zip_extract(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Zip.extract() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let zip_path = get_string_arg(&args[0], "path")?;
    let output_dir = get_string_arg(&args[1], "output_dir")?;

    let file = File::open(&zip_path)
        .map_err(|e| format!("failed to open zip file '{}': {}", zip_path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("failed to read zip archive '{}': {}", zip_path, e))?;

    let output_path = Path::new(&output_dir);
    fs::create_dir_all(output_path)
        .map_err(|e| format!("failed to create output directory '{}': {}", output_dir, e))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("failed to read entry {}: {}", i, e))?;

        let entry_path = output_path.join(entry.name());

        if entry.is_dir() {
            fs::create_dir_all(&entry_path)
                .map_err(|e| format!("failed to create directory {:?}: {}", entry_path, e))?;
        } else {
            if let Some(parent) = entry_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!("failed to create parent directory {:?}: {}", parent, e)
                })?;
            }
            let mut outfile = File::create(&entry_path)
                .map_err(|e| format!("failed to create file {:?}: {}", entry_path, e))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("failed to extract file {:?}: {}", entry_path, e))?;
        }
    }

    Ok(Value::Null)
}

/// Zip.extract_file(zip_path: String, entry_name: String, output_path: String) -> nil
/// Extracts a single file from a zip archive
fn zip_extract_file(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err(format!(
            "Zip.extract_file() expects 3 arguments, got {}",
            args.len()
        ));
    }
    let zip_path = get_string_arg(&args[0], "zip_path")?;
    let entry_name = get_string_arg(&args[1], "entry_name")?;
    let output_path = get_string_arg(&args[2], "output_path")?;

    let file = File::open(&zip_path)
        .map_err(|e| format!("failed to open zip file '{}': {}", zip_path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("failed to read zip archive '{}': {}", zip_path, e))?;

    let mut entry = archive
        .by_name(&entry_name)
        .map_err(|e| format!("entry '{}' not found in archive: {}", entry_name, e))?;

    let out_path = Path::new(&output_path);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create parent directory {:?}: {}", parent, e))?;
    }

    let mut outfile = File::create(out_path)
        .map_err(|e| format!("failed to create file {:?}: {}", out_path, e))?;
    std::io::copy(&mut entry, &mut outfile)
        .map_err(|e| format!("failed to extract file '{}': {}", entry_name, e))?;

    Ok(Value::Null)
}

/// Zip.create(output_path: String, files: List<String>) -> nil
/// Creates a new zip archive from a list of files
fn zip_create(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Zip.create() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let output_path = get_string_arg(&args[0], "output_path")?;
    let files = match &args[1] {
        Value::List(list) => list.borrow().clone(),
        _ => return Err(format!("files must be List, got {}", args[1].type_name())),
    };

    let zip_file = File::create(&output_path)
        .map_err(|e| format!("failed to create zip file '{}': {}", output_path, e))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for file_val in files {
        let file_path = get_string_arg(&file_val, "file")?;
        let path = Path::new(&file_path);

        if !path.exists() {
            return Err(format!("file not found: '{}'", file_path));
        }

        // Use the file name as the entry name in the archive
        let entry_name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.clone());

        zip_writer
            .start_file(&entry_name, options)
            .map_err(|e| format!("failed to add '{}' to archive: {}", entry_name, e))?;

        let content =
            fs::read(path).map_err(|e| format!("failed to read file '{}': {}", file_path, e))?;
        zip_writer
            .write_all(&content)
            .map_err(|e| format!("failed to write '{}' to archive: {}", entry_name, e))?;
    }

    zip_writer
        .finish()
        .map_err(|e| format!("failed to finalize zip archive: {}", e))?;

    Ok(Value::Null)
}

/// Zip.read_text(zip_path: String, entry_name: String) -> String
/// Reads a file from a zip archive as text
fn zip_read_text(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Zip.read_text() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let zip_path = get_string_arg(&args[0], "zip_path")?;
    let entry_name = get_string_arg(&args[1], "entry_name")?;

    let file = File::open(&zip_path)
        .map_err(|e| format!("failed to open zip file '{}': {}", zip_path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("failed to read zip archive '{}': {}", zip_path, e))?;

    let mut entry = archive
        .by_name(&entry_name)
        .map_err(|e| format!("entry '{}' not found in archive: {}", entry_name, e))?;

    let mut content = String::new();
    entry
        .read_to_string(&mut content)
        .map_err(|e| format!("failed to read entry '{}': {}", entry_name, e))?;

    Ok(Value::string(content))
}

/// Zip.read_bytes(zip_path: String, entry_name: String) -> List<Int>
/// Reads a file from a zip archive as bytes
fn zip_read_bytes(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Zip.read_bytes() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let zip_path = get_string_arg(&args[0], "zip_path")?;
    let entry_name = get_string_arg(&args[1], "entry_name")?;

    let file = File::open(&zip_path)
        .map_err(|e| format!("failed to open zip file '{}': {}", zip_path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("failed to read zip archive '{}': {}", zip_path, e))?;

    let mut entry = archive
        .by_name(&entry_name)
        .map_err(|e| format!("entry '{}' not found in archive: {}", entry_name, e))?;

    let mut content = Vec::new();
    entry
        .read_to_end(&mut content)
        .map_err(|e| format!("failed to read entry '{}': {}", entry_name, e))?;

    Ok(bytes_to_list(&content))
}

// ============================================================================
// Hash Module
// ============================================================================

/// Hash module entry point
pub fn hash_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "sha256" => hash_sha256(args),
        "sha256_bytes" => hash_sha256_bytes(args),
        "sha512" => hash_sha512(args),
        "sha512_bytes" => hash_sha512_bytes(args),
        "md5" => hash_md5(args),
        "md5_bytes" => hash_md5_bytes(args),
        "hmac_sha256" => hash_hmac_sha256(args),
        "hmac_sha512" => hash_hmac_sha512(args),
        _ => Err(format!("Hash has no method '{method}'")),
    }
}

/// Hash.sha256(data: String) -> String
/// Returns hex-encoded SHA-256 hash
fn hash_sha256(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Hash.sha256() expects 1 argument, got {}",
            args.len()
        ));
    }
    let data = get_string_arg(&args[0], "data")?;
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    Ok(Value::string(hex::encode(result)))
}

/// Hash.sha256_bytes(data: List<Int>) -> String
/// Hash raw bytes and return hex-encoded result
fn hash_sha256_bytes(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Hash.sha256_bytes() expects 1 argument, got {}",
            args.len()
        ));
    }
    let bytes = get_bytes_arg(&args[0])?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    Ok(Value::string(hex::encode(result)))
}

/// Hash.sha512(data: String) -> String
fn hash_sha512(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Hash.sha512() expects 1 argument, got {}",
            args.len()
        ));
    }
    let data = get_string_arg(&args[0], "data")?;
    let mut hasher = Sha512::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    Ok(Value::string(hex::encode(result)))
}

/// Hash.sha512_bytes(data: List<Int>) -> String
fn hash_sha512_bytes(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Hash.sha512_bytes() expects 1 argument, got {}",
            args.len()
        ));
    }
    let bytes = get_bytes_arg(&args[0])?;
    let mut hasher = Sha512::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    Ok(Value::string(hex::encode(result)))
}

/// Hash.md5(data: String) -> String
/// Note: MD5 is for compatibility, not security
fn hash_md5(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Hash.md5() expects 1 argument, got {}", args.len()));
    }
    let data = get_string_arg(&args[0], "data")?;
    let mut hasher = Md5::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    Ok(Value::string(hex::encode(result)))
}

/// Hash.md5_bytes(data: List<Int>) -> String
fn hash_md5_bytes(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Hash.md5_bytes() expects 1 argument, got {}",
            args.len()
        ));
    }
    let bytes = get_bytes_arg(&args[0])?;
    let mut hasher = Md5::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    Ok(Value::string(hex::encode(result)))
}

/// Hash.hmac_sha256(key: String, message: String) -> String
fn hash_hmac_sha256(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Hash.hmac_sha256() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let key = get_string_arg(&args[0], "key")?;
    let message = get_string_arg(&args[1], "message")?;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key.as_bytes())
        .map_err(|e| format!("HMAC key error: {e}"))?;
    Mac::update(&mut mac, message.as_bytes());
    let result = mac.finalize();
    Ok(Value::string(hex::encode(result.into_bytes())))
}

/// Hash.hmac_sha512(key: String, message: String) -> String
fn hash_hmac_sha512(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Hash.hmac_sha512() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let key = get_string_arg(&args[0], "key")?;
    let message = get_string_arg(&args[1], "message")?;

    type HmacSha512 = Hmac<Sha512>;
    let mut mac = <HmacSha512 as Mac>::new_from_slice(key.as_bytes())
        .map_err(|e| format!("HMAC key error: {e}"))?;
    Mac::update(&mut mac, message.as_bytes());
    let result = mac.finalize();
    Ok(Value::string(hex::encode(result.into_bytes())))
}

// ============================================================================
// Crypto Module (Advanced Cryptography)
// ============================================================================

/// Crypto module entry point
pub fn crypto_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "random_bytes" => crypto_random_bytes(args),
        "aes_encrypt" => crypto_aes_encrypt(args),
        "aes_decrypt" => crypto_aes_decrypt(args),
        "pbkdf2" => crypto_pbkdf2(args),
        _ => Err(format!("Crypto has no method '{method}'")),
    }
}

/// Crypto.random_bytes(n: Int) -> List<Int>
/// Generates cryptographically secure random bytes using the OS random number generator.
fn crypto_random_bytes(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Crypto.random_bytes() expects 1 argument, got {}",
            args.len()
        ));
    }
    let n = get_int_arg(&args[0], "n")?;
    if n < 0 {
        return Err(format!(
            "Crypto.random_bytes(): n must be non-negative, got {n}"
        ));
    }
    if n > 1_000_000 {
        return Err("Crypto.random_bytes(): n too large (max 1000000)".to_string());
    }

    use rand::RngCore;
    let mut bytes = vec![0u8; n as usize];
    OsRng.fill_bytes(&mut bytes);
    let values: Vec<Value> = bytes
        .into_iter()
        .map(|b| Value::Int(i64::from(b)))
        .collect();
    Ok(Value::list(values))
}

/// Crypto.aes_encrypt(data: String, key: String) -> String
/// Encrypts data using AES-256-GCM authenticated encryption.
/// Key must be 32 bytes (256 bits) - can be derived using Crypto.pbkdf2().
/// Returns base64-encoded ciphertext (nonce + ciphertext + auth tag).
fn crypto_aes_encrypt(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Crypto.aes_encrypt() expects 2 arguments (data, key), got {}",
            args.len()
        ));
    }
    let data = get_string_arg(&args[0], "data")?;
    let key_str = get_string_arg(&args[1], "key")?;

    // Key can be hex-encoded (64 chars) or raw bytes as string (32 chars)
    let key_bytes = if key_str.len() == 64 && key_str.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(&key_str).map_err(|e| format!("Invalid hex key: {e}"))?
    } else if key_str.len() == 32 {
        key_str.as_bytes().to_vec()
    } else {
        return Err(format!(
            "Crypto.aes_encrypt(): key must be 32 bytes (raw) or 64 hex chars, got {} chars",
            key_str.len()
        ));
    };

    if key_bytes.len() != 32 {
        return Err(format!(
            "Crypto.aes_encrypt(): key must be exactly 32 bytes, got {}",
            key_bytes.len()
        ));
    }

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, data.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))?;

    // Prepend nonce to ciphertext for storage (nonce is 12 bytes)
    let mut result = nonce.to_vec();
    result.extend(ciphertext);

    // Return base64-encoded result
    Ok(Value::string(
        base64::engine::general_purpose::STANDARD.encode(&result),
    ))
}

/// Crypto.aes_decrypt(encrypted: String, key: String) -> String
/// Decrypts data encrypted with Crypto.aes_encrypt().
/// Key must be the same 32-byte key used for encryption.
fn crypto_aes_decrypt(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Crypto.aes_decrypt() expects 2 arguments (encrypted, key), got {}",
            args.len()
        ));
    }
    let encrypted_b64 = get_string_arg(&args[0], "encrypted")?;
    let key_str = get_string_arg(&args[1], "key")?;

    // Decode base64 input
    let encrypted = base64::engine::general_purpose::STANDARD
        .decode(&encrypted_b64)
        .map_err(|e| format!("Invalid base64 input: {e}"))?;

    // Key can be hex-encoded (64 chars) or raw bytes as string (32 chars)
    let key_bytes = if key_str.len() == 64 && key_str.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(&key_str).map_err(|e| format!("Invalid hex key: {e}"))?
    } else if key_str.len() == 32 {
        key_str.as_bytes().to_vec()
    } else {
        return Err(format!(
            "Crypto.aes_decrypt(): key must be 32 bytes (raw) or 64 hex chars, got {} chars",
            key_str.len()
        ));
    };

    if key_bytes.len() != 32 {
        return Err(format!(
            "Crypto.aes_decrypt(): key must be exactly 32 bytes, got {}",
            key_bytes.len()
        ));
    }

    // Nonce is first 12 bytes
    if encrypted.len() < 12 {
        return Err("Crypto.aes_decrypt(): encrypted data too short".to_string());
    }
    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption failed: invalid key or corrupted data".to_string())?;

    String::from_utf8(plaintext)
        .map(Value::string)
        .map_err(|e| format!("Decrypted data is not valid UTF-8: {e}"))
}

/// Crypto.pbkdf2(password: String, salt: String, iterations: Int) -> String
/// Derives a 256-bit key from a password using PBKDF2-HMAC-SHA256.
/// Returns hex-encoded 32-byte key suitable for use with aes_encrypt/aes_decrypt.
fn crypto_pbkdf2(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err(format!(
            "Crypto.pbkdf2() expects 3 arguments (password, salt, iterations), got {}",
            args.len()
        ));
    }
    let password = get_string_arg(&args[0], "password")?;
    let salt = get_string_arg(&args[1], "salt")?;
    let iterations = get_int_arg(&args[2], "iterations")?;

    if iterations < 1 {
        return Err("Crypto.pbkdf2(): iterations must be at least 1".to_string());
    }
    if iterations > 10_000_000 {
        return Err("Crypto.pbkdf2(): iterations too high (max 10000000)".to_string());
    }

    // Derive 32-byte (256-bit) key using PBKDF2-HMAC-SHA256
    let key: [u8; 32] =
        pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), salt.as_bytes(), iterations as u32);

    Ok(Value::string(hex::encode(key)))
}

// ============================================================================
// Uuid Module
// ============================================================================

/// Uuid module entry point
pub fn uuid_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "v4" => uuid_v4(args),
        "v7" => uuid_v7(args),
        "parse" => uuid_parse(args),
        "is_valid" => uuid_is_valid(args),
        _ => Err(format!("Uuid has no method '{method}'")),
    }
}

/// Uuid.v4() -> String
/// Generate a random UUID (version 4)
fn uuid_v4(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!("Uuid.v4() expects 0 arguments, got {}", args.len()));
    }
    let id = Uuid::new_v4();
    Ok(Value::string(id.to_string()))
}

/// Uuid.v7() -> String
/// Generate a time-based sortable UUID (version 7)
fn uuid_v7(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!("Uuid.v7() expects 0 arguments, got {}", args.len()));
    }
    let id = Uuid::now_v7();
    Ok(Value::string(id.to_string()))
}

/// Uuid.parse(str: String) -> String
/// Parse and normalize a UUID string (returns canonical lowercase format)
fn uuid_parse(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Uuid.parse() expects 1 argument, got {}",
            args.len()
        ));
    }
    let s = get_string_arg(&args[0], "uuid")?;
    let id = Uuid::parse_str(&s).map_err(|e| format!("invalid UUID: {e}"))?;
    Ok(Value::string(id.to_string()))
}

/// Uuid.is_valid(str: String) -> Bool
/// Check if a string is a valid UUID
fn uuid_is_valid(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Uuid.is_valid() expects 1 argument, got {}",
            args.len()
        ));
    }
    let s = get_string_arg(&args[0], "uuid")?;
    Ok(Value::Bool(Uuid::parse_str(&s).is_ok()))
}

// ============================================================================
// Random Module
// ============================================================================

/// Random module entry point
pub fn random_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "int" => random_int(args),
        "float" => random_float(args),
        "bool" => random_bool(args),
        "choice" => random_choice(args),
        "shuffle" => random_shuffle(args),
        "bytes" => random_bytes(args),
        _ => Err(format!("Random has no method '{method}'")),
    }
}

/// Random.int(min: Int, max: Int) -> Int
/// Generate a random integer in range [min, max] (inclusive)
fn random_int(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Random.int() expects 2 arguments (min, max), got {}",
            args.len()
        ));
    }
    let min = get_int_arg(&args[0], "min")?;
    let max = get_int_arg(&args[1], "max")?;

    if min > max {
        return Err(format!("Random.int(): min ({min}) must be <= max ({max})"));
    }

    let mut rng = rand::thread_rng();
    let result = rng.gen_range(min..=max);
    Ok(Value::Int(result))
}

/// Random.float() -> Float
/// Generate a random float in range [0.0, 1.0)
fn random_float(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "Random.float() expects 0 arguments, got {}",
            args.len()
        ));
    }
    let mut rng = rand::thread_rng();
    Ok(Value::Float(rng.gen()))
}

/// Random.bool() -> Bool
/// Generate a random boolean
fn random_bool(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "Random.bool() expects 0 arguments, got {}",
            args.len()
        ));
    }
    let mut rng = rand::thread_rng();
    Ok(Value::Bool(rng.gen()))
}

/// Random.choice(list: List<T>) -> T
/// Pick a random element from a list
fn random_choice(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Random.choice() expects 1 argument, got {}",
            args.len()
        ));
    }

    let list = match &args[0] {
        Value::List(l) => l.borrow(),
        _ => {
            return Err(format!(
                "Random.choice() expects List, got {}",
                args[0].type_name()
            ))
        }
    };

    if list.is_empty() {
        return Err("Random.choice(): cannot choose from empty list".to_string());
    }

    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..list.len());
    Ok(list[index].clone())
}

/// Random.shuffle(list: List<T>) -> List<T>
/// Return a new list with elements in random order
fn random_shuffle(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Random.shuffle() expects 1 argument, got {}",
            args.len()
        ));
    }

    let list = match &args[0] {
        Value::List(l) => l.borrow().clone(),
        _ => {
            return Err(format!(
                "Random.shuffle() expects List, got {}",
                args[0].type_name()
            ))
        }
    };

    let mut shuffled = list;
    let mut rng = rand::thread_rng();

    // Fisher-Yates shuffle
    for i in (1..shuffled.len()).rev() {
        let j = rng.gen_range(0..=i);
        shuffled.swap(i, j);
    }

    Ok(Value::List(Rc::new(RefCell::new(shuffled))))
}

/// Random.bytes(n: Int) -> List<Int>
/// Generate n random bytes
fn random_bytes(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Random.bytes() expects 1 argument, got {}",
            args.len()
        ));
    }
    let n = get_int_arg(&args[0], "n")?;
    if n < 0 {
        return Err(format!("Random.bytes(): n must be non-negative, got {n}"));
    }
    if n > 1_000_000 {
        return Err("Random.bytes(): n too large (max 1000000)".to_string());
    }

    let mut rng = rand::thread_rng();
    let bytes: Vec<Value> = (0..n)
        .map(|_| Value::Int(i64::from(rng.gen::<u8>())))
        .collect();

    Ok(Value::List(Rc::new(RefCell::new(bytes))))
}

// ============================================================================
// Math Module - Mathematical constants and functions
// ============================================================================

/// Math module entry point
pub fn math_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        // Constants
        "pi" | "PI" => Ok(Value::Float(std::f64::consts::PI)),
        "e" | "E" => Ok(Value::Float(std::f64::consts::E)),
        "tau" | "TAU" => Ok(Value::Float(std::f64::consts::TAU)),
        "infinity" | "INFINITY" => Ok(Value::Float(f64::INFINITY)),
        "neg_infinity" | "NEG_INFINITY" => Ok(Value::Float(f64::NEG_INFINITY)),
        "nan" | "NAN" => Ok(Value::Float(f64::NAN)),

        // Basic functions
        "abs" => math_abs(args),
        "floor" => math_floor(args),
        "ceil" => math_ceil(args),
        "round" => math_round(args),
        "trunc" => math_trunc(args),
        "sign" | "signum" => math_sign(args),
        "fract" => math_fract(args),

        // Trigonometric functions
        "sin" => math_sin(args),
        "cos" => math_cos(args),
        "tan" => math_tan(args),
        "asin" => math_asin(args),
        "acos" => math_acos(args),
        "atan" => math_atan(args),
        "atan2" => math_atan2(args),
        "sinh" => math_sinh(args),
        "cosh" => math_cosh(args),
        "tanh" => math_tanh(args),

        // Exponential and logarithmic functions
        "exp" => math_exp(args),
        "exp2" => math_exp2(args),
        "ln" | "log" => math_ln(args),
        "log2" => math_log2(args),
        "log10" => math_log10(args),
        "pow" => math_pow(args),
        "sqrt" => math_sqrt(args),
        "cbrt" => math_cbrt(args),

        // Utility functions
        "min" => math_min(args),
        "max" => math_max(args),
        "clamp" => math_clamp(args),
        "hypot" => math_hypot(args),

        // Angle conversions
        "degrees" | "to_degrees" => math_to_degrees(args),
        "radians" | "to_radians" => math_to_radians(args),

        // Validation
        "is_nan" => math_is_nan(args),
        "is_infinite" => math_is_infinite(args),
        "is_finite" => math_is_finite(args),

        // List aggregate functions
        "sum" => math_sum(args),
        "mean" => math_mean(args),
        "median" => math_median(args),
        "std" => math_std(args),
        "variance" => math_variance(args),

        // Rounding with precision
        "round_to" => math_round_to(args),

        _ => Err(format!("Math has no method '{method}'")),
    }
}

// Helper to get a float argument (accepts Int or Float)
fn get_float_arg_math(arg: &Value, name: &str) -> Result<f64, String> {
    match arg {
        Value::Int(n) => Ok(*n as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(format!(
            "{name} must be a number (Int or Float), got {}",
            arg.type_name()
        )),
    }
}

/// Math.abs(x) -> number
fn math_abs(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Math.abs() expects 1 argument, got {}", args.len()));
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n.abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err(format!(
            "Math.abs() expects number, got {}",
            args[0].type_name()
        )),
    }
}

/// Math.floor(x) -> Int
fn math_floor(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.floor() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Int(x.floor() as i64))
}

/// Math.ceil(x) -> Int
fn math_ceil(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.ceil() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Int(x.ceil() as i64))
}

/// Math.round(x) -> Int
fn math_round(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.round() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Int(x.round() as i64))
}

/// Math.trunc(x) -> Int
fn math_trunc(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.trunc() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Int(x.trunc() as i64))
}

/// Math.sign(x) -> Int (-1, 0, or 1)
fn math_sign(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.sign() expects 1 argument, got {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n.signum())),
        Value::Float(f) => {
            if f.is_nan() {
                Ok(Value::Float(f64::NAN))
            } else if *f > 0.0 {
                Ok(Value::Int(1))
            } else if *f < 0.0 {
                Ok(Value::Int(-1))
            } else {
                Ok(Value::Int(0))
            }
        }
        _ => Err(format!(
            "Math.sign() expects number, got {}",
            args[0].type_name()
        )),
    }
}

/// Math.fract(x) -> Float (fractional part)
fn math_fract(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.fract() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.fract()))
}

// Trigonometric functions

/// Math.sin(x) -> Float
fn math_sin(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Math.sin() expects 1 argument, got {}", args.len()));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.sin()))
}

/// Math.cos(x) -> Float
fn math_cos(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Math.cos() expects 1 argument, got {}", args.len()));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.cos()))
}

/// Math.tan(x) -> Float
fn math_tan(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Math.tan() expects 1 argument, got {}", args.len()));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.tan()))
}

/// Math.asin(x) -> Float
fn math_asin(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.asin() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.asin()))
}

/// Math.acos(x) -> Float
fn math_acos(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.acos() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.acos()))
}

/// Math.atan(x) -> Float
fn math_atan(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.atan() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.atan()))
}

/// Math.atan2(y, x) -> Float
fn math_atan2(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Math.atan2() expects 2 arguments (y, x), got {}",
            args.len()
        ));
    }
    let y = get_float_arg_math(&args[0], "y")?;
    let x = get_float_arg_math(&args[1], "x")?;
    Ok(Value::Float(y.atan2(x)))
}

/// Math.sinh(x) -> Float
fn math_sinh(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.sinh() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.sinh()))
}

/// Math.cosh(x) -> Float
fn math_cosh(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.cosh() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.cosh()))
}

/// Math.tanh(x) -> Float
fn math_tanh(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.tanh() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.tanh()))
}

// Exponential and logarithmic functions

/// Math.exp(x) -> Float (e^x)
fn math_exp(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Math.exp() expects 1 argument, got {}", args.len()));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.exp()))
}

/// Math.exp2(x) -> Float (2^x)
fn math_exp2(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.exp2() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.exp2()))
}

/// Math.ln(x) -> Float (natural log)
fn math_ln(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Math.ln() expects 1 argument, got {}", args.len()));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.ln()))
}

/// Math.log2(x) -> Float
fn math_log2(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.log2() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.log2()))
}

/// Math.log10(x) -> Float
fn math_log10(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.log10() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.log10()))
}

/// Math.pow(base, exp) -> Float
fn math_pow(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Math.pow() expects 2 arguments (base, exp), got {}",
            args.len()
        ));
    }
    let base = get_float_arg_math(&args[0], "base")?;
    let exp = get_float_arg_math(&args[1], "exp")?;
    Ok(Value::Float(base.powf(exp)))
}

/// Math.sqrt(x) -> Float
fn math_sqrt(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.sqrt() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.sqrt()))
}

/// Math.cbrt(x) -> Float (cube root)
fn math_cbrt(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.cbrt() expects 1 argument, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    Ok(Value::Float(x.cbrt()))
}

// Utility functions

/// Math.min(a, b, ...) -> number
fn math_min(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("Math.min() expects at least 1 argument".to_string());
    }

    let mut result = get_float_arg_math(&args[0], "arg")?;
    let mut is_int = matches!(&args[0], Value::Int(_));

    for arg in &args[1..] {
        let val = get_float_arg_math(arg, "arg")?;
        if val < result {
            result = val;
            is_int = matches!(arg, Value::Int(_));
        }
    }

    if is_int && result.fract() == 0.0 && result >= i64::MIN as f64 && result <= i64::MAX as f64 {
        Ok(Value::Int(result as i64))
    } else {
        Ok(Value::Float(result))
    }
}

/// Math.max(a, b, ...) -> number
fn math_max(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("Math.max() expects at least 1 argument".to_string());
    }

    let mut result = get_float_arg_math(&args[0], "arg")?;
    let mut is_int = matches!(&args[0], Value::Int(_));

    for arg in &args[1..] {
        let val = get_float_arg_math(arg, "arg")?;
        if val > result {
            result = val;
            is_int = matches!(arg, Value::Int(_));
        }
    }

    if is_int && result.fract() == 0.0 && result >= i64::MIN as f64 && result <= i64::MAX as f64 {
        Ok(Value::Int(result as i64))
    } else {
        Ok(Value::Float(result))
    }
}

/// Math.clamp(value, min, max) -> number
fn math_clamp(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err(format!(
            "Math.clamp() expects 3 arguments (value, min, max), got {}",
            args.len()
        ));
    }
    let value = get_float_arg_math(&args[0], "value")?;
    let min_val = get_float_arg_math(&args[1], "min")?;
    let max_val = get_float_arg_math(&args[2], "max")?;

    if min_val > max_val {
        return Err(format!(
            "Math.clamp(): min ({min_val}) must be <= max ({max_val})"
        ));
    }

    let result = value.clamp(min_val, max_val);

    // Preserve Int type if all inputs were Int and result is whole
    if matches!(
        (&args[0], &args[1], &args[2]),
        (Value::Int(_), Value::Int(_), Value::Int(_))
    ) && result.fract() == 0.0
        && result >= i64::MIN as f64
        && result <= i64::MAX as f64
    {
        Ok(Value::Int(result as i64))
    } else {
        Ok(Value::Float(result))
    }
}

/// Math.hypot(x, y) -> Float (sqrt(x^2 + y^2))
fn math_hypot(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Math.hypot() expects 2 arguments (x, y), got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    let y = get_float_arg_math(&args[1], "y")?;
    Ok(Value::Float(x.hypot(y)))
}

// Angle conversions

/// Math.to_degrees(radians) -> Float
fn math_to_degrees(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.to_degrees() expects 1 argument, got {}",
            args.len()
        ));
    }
    let radians = get_float_arg_math(&args[0], "radians")?;
    Ok(Value::Float(radians.to_degrees()))
}

/// Math.to_radians(degrees) -> Float
fn math_to_radians(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.to_radians() expects 1 argument, got {}",
            args.len()
        ));
    }
    let degrees = get_float_arg_math(&args[0], "degrees")?;
    Ok(Value::Float(degrees.to_radians()))
}

// Validation functions

/// Math.is_nan(x) -> Bool
fn math_is_nan(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.is_nan() expects 1 argument, got {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::Float(f) => Ok(Value::Bool(f.is_nan())),
        Value::Int(_) => Ok(Value::Bool(false)),
        _ => Err(format!(
            "Math.is_nan() expects number, got {}",
            args[0].type_name()
        )),
    }
}

/// Math.is_infinite(x) -> Bool
fn math_is_infinite(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.is_infinite() expects 1 argument, got {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::Float(f) => Ok(Value::Bool(f.is_infinite())),
        Value::Int(_) => Ok(Value::Bool(false)),
        _ => Err(format!(
            "Math.is_infinite() expects number, got {}",
            args[0].type_name()
        )),
    }
}

/// Math.is_finite(x) -> Bool
fn math_is_finite(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.is_finite() expects 1 argument, got {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::Float(f) => Ok(Value::Bool(f.is_finite())),
        Value::Int(_) => Ok(Value::Bool(true)), // Integers are always finite
        _ => Err(format!(
            "Math.is_finite() expects number, got {}",
            args[0].type_name()
        )),
    }
}

// List aggregate functions

/// Helper to extract a list of numbers from a Value::List
fn get_number_list(arg: &Value) -> Result<Vec<f64>, String> {
    match arg {
        Value::List(list) => list
            .borrow()
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as f64),
                Value::Float(f) => Ok(*f),
                _ => Err(format!(
                    "list element must be a number, got {}",
                    v.type_name()
                )),
            })
            .collect(),
        _ => Err(format!("expected List, got {}", arg.type_name())),
    }
}

/// Math.sum(list) -> Float
/// Sum all numbers in a list
fn math_sum(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Math.sum() expects 1 argument, got {}", args.len()));
    }
    let numbers = get_number_list(&args[0])?;
    Ok(Value::Float(numbers.iter().sum()))
}

/// Math.mean(list) -> Float
/// Calculate the arithmetic mean (average) of numbers in a list
fn math_mean(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.mean() expects 1 argument, got {}",
            args.len()
        ));
    }
    let numbers = get_number_list(&args[0])?;
    if numbers.is_empty() {
        return Ok(Value::Float(f64::NAN));
    }
    let sum: f64 = numbers.iter().sum();
    Ok(Value::Float(sum / numbers.len() as f64))
}

/// Math.median(list) -> Float
/// Calculate the median of numbers in a list
fn math_median(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.median() expects 1 argument, got {}",
            args.len()
        ));
    }
    let mut numbers = get_number_list(&args[0])?;
    if numbers.is_empty() {
        return Ok(Value::Float(f64::NAN));
    }
    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = numbers.len();
    let median = if len % 2 == 0 {
        (numbers[len / 2 - 1] + numbers[len / 2]) / 2.0
    } else {
        numbers[len / 2]
    };
    Ok(Value::Float(median))
}

/// Math.variance(list) -> Float
/// Calculate the population variance of numbers in a list
fn math_variance(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Math.variance() expects 1 argument, got {}",
            args.len()
        ));
    }
    let numbers = get_number_list(&args[0])?;
    if numbers.is_empty() {
        return Ok(Value::Float(f64::NAN));
    }
    let mean: f64 = numbers.iter().sum::<f64>() / numbers.len() as f64;
    let variance = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / numbers.len() as f64;
    Ok(Value::Float(variance))
}

/// Math.std(list) -> Float
/// Calculate the population standard deviation of numbers in a list
fn math_std(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Math.std() expects 1 argument, got {}", args.len()));
    }
    let numbers = get_number_list(&args[0])?;
    if numbers.is_empty() {
        return Ok(Value::Float(f64::NAN));
    }
    let mean: f64 = numbers.iter().sum::<f64>() / numbers.len() as f64;
    let variance = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / numbers.len() as f64;
    Ok(Value::Float(variance.sqrt()))
}

/// Math.round_to(x, decimals) -> Float
/// Round a number to the specified number of decimal places
fn math_round_to(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Math.round_to() expects 2 arguments, got {}",
            args.len()
        ));
    }
    let x = get_float_arg_math(&args[0], "x")?;
    let decimals = match &args[1] {
        Value::Int(n) => *n,
        _ => {
            return Err(format!(
                "Math.round_to() decimals must be Int, got {}",
                args[1].type_name()
            ))
        }
    };
    if decimals < 0 {
        return Err("Math.round_to() decimals must be non-negative".to_string());
    }
    let multiplier = 10_f64.powi(decimals as i32);
    Ok(Value::Float((x * multiplier).round() / multiplier))
}

// ============================================================================
// Input Module
// ============================================================================

pub fn input_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "read_line" => input_read_line(args),
        "read_all" => input_read_all(args),
        "prompt" => input_prompt(args),
        "prompt_int" => input_prompt_int(args),
        "prompt_bool" => input_prompt_bool(args),
        "prompt_secret" => input_prompt_secret(args),
        "choose" => input_choose(args),
        _ => Err(format!("Input has no method '{method}'")),
    }
}

/// Input.read_line() -> String
/// Read a single line from stdin
fn input_read_line(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "Input.read_line() expects 0 arguments, got {}",
            args.len()
        ));
    }

    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| format!("failed to read line from stdin: {e}"))?;

    // Remove trailing newline
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }

    Ok(Value::string(line))
}

/// Input.read_all() -> String
/// Read all input from stdin until EOF
fn input_read_all(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "Input.read_all() expects 0 arguments, got {}",
            args.len()
        ));
    }

    use std::io::Read;
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|e| format!("failed to read from stdin: {e}"))?;

    Ok(Value::string(buffer))
}

/// Input.prompt(message: String) -> String
/// Display a prompt message and return user input
fn input_prompt(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Input.prompt() expects 1 argument, got {}",
            args.len()
        ));
    }

    let prompt_msg = get_string_arg(&args[0], "message")?;

    use std::io::BufRead;
    print!("{prompt_msg}");
    std::io::stdout()
        .flush()
        .map_err(|e| format!("failed to flush stdout: {e}"))?;

    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| format!("failed to read line from stdin: {e}"))?;

    // Remove trailing newline
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }

    Ok(Value::string(line))
}

/// Input.prompt_int(message: String) -> Int
/// Display a prompt message and parse the input as an integer
fn input_prompt_int(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Input.prompt_int() expects 1 argument, got {}",
            args.len()
        ));
    }

    let prompt_msg = get_string_arg(&args[0], "message")?;

    use std::io::BufRead;
    print!("{prompt_msg}");
    std::io::stdout()
        .flush()
        .map_err(|e| format!("failed to flush stdout: {e}"))?;

    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| format!("failed to read line from stdin: {e}"))?;

    let trimmed = line.trim();
    let value: i64 = trimmed
        .parse()
        .map_err(|_| format!("invalid integer: '{trimmed}'"))?;

    Ok(Value::Int(value))
}

/// Input.prompt_bool(message: String) -> Bool
/// Display a prompt message and parse the input as a boolean
/// Accepts: y, yes, true, 1 (true) | n, no, false, 0 (false) - case insensitive
fn input_prompt_bool(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Input.prompt_bool() expects 1 argument, got {}",
            args.len()
        ));
    }

    let prompt_msg = get_string_arg(&args[0], "message")?;

    use std::io::BufRead;
    print!("{prompt_msg}");
    std::io::stdout()
        .flush()
        .map_err(|e| format!("failed to flush stdout: {e}"))?;

    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| format!("failed to read line from stdin: {e}"))?;

    let trimmed = line.trim().to_lowercase();
    match trimmed.as_str() {
        "y" | "yes" | "true" | "1" => Ok(Value::Bool(true)),
        "n" | "no" | "false" | "0" => Ok(Value::Bool(false)),
        _ => Err(format!(
            "invalid boolean: '{trimmed}' (expected: y/yes/true/1 or n/no/false/0)"
        )),
    }
}

/// Input.prompt_secret(message: String) -> String
/// Display a prompt message and read hidden input (for passwords)
fn input_prompt_secret(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Input.prompt_secret() expects 1 argument, got {}",
            args.len()
        ));
    }

    let prompt_msg = get_string_arg(&args[0], "message")?;

    let password = rpassword::prompt_password(&prompt_msg)
        .map_err(|e| format!("failed to read secret input: {e}"))?;

    Ok(Value::string(password))
}

/// Input.choose(message: String, options: List<String>) -> String
/// Display a numbered list of options and return the selected one
fn input_choose(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Input.choose() expects 2 arguments, got {}",
            args.len()
        ));
    }

    let prompt_msg = get_string_arg(&args[0], "message")?;

    let options = match &args[1] {
        Value::List(list) => list.borrow().clone(),
        _ => {
            return Err(format!(
                "Input.choose() options must be List, got {}",
                args[1].type_name()
            ))
        }
    };

    if options.is_empty() {
        return Err("Input.choose(): options list cannot be empty".to_string());
    }

    // Convert options to strings
    let string_options: Vec<String> = options
        .iter()
        .map(|v| match v {
            Value::String(s) => Ok(s.to_string()),
            _ => Err(format!(
                "Input.choose() options must be strings, got {}",
                v.type_name()
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Display prompt and options
    println!("{prompt_msg}");
    for (i, opt) in string_options.iter().enumerate() {
        println!("  {}. {opt}", i + 1);
    }

    use std::io::BufRead;
    print!("Enter choice (1-{}): ", string_options.len());
    std::io::stdout()
        .flush()
        .map_err(|e| format!("failed to flush stdout: {e}"))?;

    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| format!("failed to read line from stdin: {e}"))?;

    let trimmed = line.trim();
    let choice: usize = trimmed
        .parse()
        .map_err(|_| format!("invalid choice: '{trimmed}'"))?;

    if choice < 1 || choice > string_options.len() {
        return Err(format!(
            "choice out of range: {} (expected 1-{})",
            choice,
            string_options.len()
        ));
    }

    Ok(Value::string(string_options[choice - 1].clone()))
}

// ============================================================================
// Log Module
// ============================================================================

use std::sync::RwLock;

/// Log level for filtering messages
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl LogLevel {
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!(
                "invalid log level '{}', expected: debug, info, warn, or error",
                s
            )),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Output target for log messages
#[derive(Clone, Debug)]
enum LogOutput {
    Stdout,
    Stderr,
    File(String),
}

/// Configuration for the logging system
#[derive(Clone)]
struct LogConfig {
    level: LogLevel,
    output: LogOutput,
    format: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            output: LogOutput::Stdout,
            format: "[{level}] {timestamp} - {message}".to_string(),
        }
    }
}

/// Global log configuration
static LOG_CONFIG: RwLock<Option<LogConfig>> = RwLock::new(None);

fn get_log_config() -> LogConfig {
    let guard = LOG_CONFIG.read().unwrap();
    guard.clone().unwrap_or_default()
}

fn update_log_config<F: FnOnce(&mut LogConfig)>(f: F) {
    let mut guard = LOG_CONFIG.write().unwrap();
    let mut config = guard.take().unwrap_or_default();
    f(&mut config);
    *guard = Some(config);
}

pub fn log_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "debug" => log_message(LogLevel::Debug, args),
        "info" => log_message(LogLevel::Info, args),
        "warn" | "warning" => log_message(LogLevel::Warn, args),
        "error" => log_message(LogLevel::Error, args),
        "set_level" => log_set_level(args),
        "to_file" => log_to_file(args),
        "to_stderr" => log_to_stderr(args),
        "to_stdout" => log_to_stdout(args),
        "set_format" => log_set_format(args),
        "level" => log_get_level(args),
        _ => Err(format!("Log has no method '{method}'")),
    }
}

fn log_message(level: LogLevel, args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Log.{}() expects 1-2 arguments, got {}",
            level.as_str().to_lowercase(),
            args.len()
        ));
    }

    // Validate arguments FIRST, before level filtering
    // This ensures users get immediate feedback on invalid args
    let message = get_string_arg(&args[0], "message")?;

    // Validate optional context map
    let context: Option<HashMap<String, String>> = if args.len() == 2 {
        match &args[1] {
            Value::Map(map) => {
                let map = map.borrow();
                let mut ctx = HashMap::new();
                for (k, v) in map.iter() {
                    let key = match k {
                        HashableValue::Null => "null".to_string(),
                        HashableValue::String(s) => s.to_string(),
                        HashableValue::Int(i) => i.to_string(),
                        HashableValue::Bool(b) => b.to_string(),
                    };
                    let val = value_to_log_string(v);
                    ctx.insert(key, val);
                }
                Some(ctx)
            }
            _ => {
                return Err(format!(
                    "Log.{}() context must be a Map, got {}",
                    level.as_str().to_lowercase(),
                    args[1].type_name()
                ))
            }
        }
    } else {
        None
    };

    let config = get_log_config();

    // Check if this level should be logged (after validation)
    if level < config.level {
        return Ok(Value::Null);
    }

    // Format the log message
    let formatted = format_log_message(&config.format, level, &message, context.as_ref());

    // Write to output
    write_log_output(&config.output, &formatted)?;

    Ok(Value::Null)
}

fn value_to_log_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.to_string(),
        Value::List(l) => {
            let items: Vec<String> = l.borrow().iter().map(value_to_log_string).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Map(m) => {
            let pairs: Vec<String> = m
                .borrow()
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        HashableValue::Null => "null".to_string(),
                        HashableValue::String(s) => s.to_string(),
                        HashableValue::Int(i) => i.to_string(),
                        HashableValue::Bool(b) => b.to_string(),
                    };
                    format!("{}: {}", key, value_to_log_string(v))
                })
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        _ => format!("<{}>", v.type_name()),
    }
}

fn format_log_message(
    format: &str,
    level: LogLevel,
    message: &str,
    context: Option<&HashMap<String, String>>,
) -> String {
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string();

    let mut result = format.to_string();
    result = result.replace("{level}", level.as_str());
    result = result.replace("{timestamp}", &timestamp);
    result = result.replace("{message}", message);

    // Append context if present
    if let Some(ctx) = context {
        if !ctx.is_empty() {
            let pairs: Vec<String> = ctx.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            result.push_str(&format!(" {{{}}}", pairs.join(", ")));
        }
    }

    result
}

fn write_log_output(output: &LogOutput, message: &str) -> Result<(), String> {
    match output {
        LogOutput::Stdout => {
            println!("{message}");
            Ok(())
        }
        LogOutput::Stderr => {
            eprintln!("{message}");
            Ok(())
        }
        LogOutput::File(path) => {
            use std::fs::OpenOptions;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| format!("failed to open log file '{}': {}", path, e))?;
            writeln!(file, "{message}")
                .map_err(|e| format!("failed to write to log file '{}': {}", path, e))?;
            Ok(())
        }
    }
}

fn log_set_level(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Log.set_level() expects 1 argument, got {}",
            args.len()
        ));
    }
    let level_str = get_string_arg(&args[0], "level")?;
    let level = LogLevel::from_str(&level_str)?;
    update_log_config(|c| c.level = level);
    Ok(Value::Null)
}

fn log_get_level(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "Log.level() expects 0 arguments, got {}",
            args.len()
        ));
    }
    let config = get_log_config();
    Ok(Value::string(config.level.as_str().to_lowercase()))
}

fn log_to_file(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Log.to_file() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    update_log_config(|c| c.output = LogOutput::File(path));
    Ok(Value::Null)
}

fn log_to_stderr(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "Log.to_stderr() expects 0 arguments, got {}",
            args.len()
        ));
    }
    update_log_config(|c| c.output = LogOutput::Stderr);
    Ok(Value::Null)
}

fn log_to_stdout(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "Log.to_stdout() expects 0 arguments, got {}",
            args.len()
        ));
    }
    update_log_config(|c| c.output = LogOutput::Stdout);
    Ok(Value::Null)
}

fn log_set_format(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Log.set_format() expects 1 argument, got {}",
            args.len()
        ));
    }
    let format = get_string_arg(&args[0], "format")?;
    update_log_config(|c| c.format = format);
    Ok(Value::Null)
}

// ============================================================================
// System Module
// ============================================================================

pub fn system_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "os" => system_os(args),
        "arch" => system_arch(args),
        "cwd" => system_cwd(args),
        "set_cwd" => system_set_cwd(args),
        "temp_dir" => system_temp_dir(args),
        "temp_file" => system_temp_file(args),
        "exit" => system_exit(args),
        "cpu_count" => system_cpu_count(args),
        "total_memory" => system_total_memory(args),
        "hostname" => system_hostname(args),
        "uptime" => system_uptime(args),
        _ => Err(format!("System has no method '{method}'")),
    }
}

fn system_os(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "System.os() expects 0 arguments, got {}",
            args.len()
        ));
    }
    // Returns "macos", "linux", "windows", etc.
    let os = std::env::consts::OS;
    // Normalize "macos" from Rust's "macos" which is already lowercase
    Ok(Value::string(os))
}

fn system_arch(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "System.arch() expects 0 arguments, got {}",
            args.len()
        ));
    }
    // Returns "x86_64", "aarch64", etc.
    Ok(Value::string(std::env::consts::ARCH))
}

fn system_cwd(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "System.cwd() expects 0 arguments, got {}",
            args.len()
        ));
    }
    env::current_dir()
        .map(|p| Value::string(p.to_string_lossy()))
        .map_err(|e| format!("failed to get current directory: {}", e))
}

fn system_set_cwd(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "System.set_cwd() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;
    env::set_current_dir(&path)
        .map(|()| Value::Null)
        .map_err(|e| format!("failed to set current directory to '{}': {}", path, e))
}

fn system_temp_dir(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "System.temp_dir() expects 0 arguments, got {}",
            args.len()
        ));
    }
    Ok(Value::string(env::temp_dir().to_string_lossy()))
}

fn system_temp_file(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "System.temp_file() expects 0 arguments, got {}",
            args.len()
        ));
    }
    // Create a temporary file and return its path
    // The file persists (not auto-deleted) for user to work with
    let temp_file =
        tempfile::NamedTempFile::new().map_err(|e| format!("failed to create temp file: {}", e))?;
    // Keep the file by converting to path (prevents auto-deletion)
    let (_, path_buf) = temp_file
        .keep()
        .map_err(|e| format!("failed to persist temp file: {}", e))?;
    Ok(Value::string(path_buf.to_string_lossy()))
}

fn system_exit(args: &[Value]) -> NativeResult {
    let code = if args.is_empty() {
        0
    } else if args.len() == 1 {
        get_int_arg(&args[0], "code")? as i32
    } else {
        return Err(format!(
            "System.exit() expects 0-1 arguments, got {}",
            args.len()
        ));
    };
    std::process::exit(code);
}

fn system_cpu_count(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "System.cpu_count() expects 0 arguments, got {}",
            args.len()
        ));
    }
    // Use sysinfo for accurate physical + logical CPU count
    use sysinfo::System;
    let sys = System::new_all();
    Ok(Value::Int(sys.cpus().len() as i64))
}

fn system_total_memory(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "System.total_memory() expects 0 arguments, got {}",
            args.len()
        ));
    }
    use sysinfo::System;
    let sys = System::new_all();
    Ok(Value::Int(sys.total_memory() as i64))
}

fn system_hostname(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "System.hostname() expects 0 arguments, got {}",
            args.len()
        ));
    }
    use sysinfo::System;
    match System::host_name() {
        Some(name) => Ok(Value::string(name)),
        None => Ok(Value::Null),
    }
}

fn system_uptime(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!(
            "System.uptime() expects 0 arguments, got {}",
            args.len()
        ));
    }
    use sysinfo::System;
    Ok(Value::Int(System::uptime() as i64))
}

// ============================================================================
// Process Module
// ============================================================================

pub fn process_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "spawn" => process_spawn(args),
        "kill" => process_kill(args),
        _ => Err(format!("Process has no method '{method}'")),
    }
}

fn process_spawn(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Process.spawn() expects 1-2 arguments, got {}",
            args.len()
        ));
    }

    let program = get_string_arg(&args[0], "program")?;
    let cmd_args: Vec<String> = if args.len() == 2 {
        match &args[1] {
            Value::List(list) => list
                .borrow()
                .iter()
                .map(|v| match v {
                    Value::String(s) => Ok(s.to_string()),
                    _ => Err(format!(
                        "Process.spawn() argument must be string, got {}",
                        v.type_name()
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => {
                return Err(format!(
                    "Process.spawn() expects List as second argument, got {}",
                    args[1].type_name()
                ))
            }
        }
    } else {
        Vec::new()
    };

    let child = Command::new(&program)
        .args(&cmd_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn '{}': {}", program, e))?;

    let pid = child.id();

    // Store the child process handle for later interaction
    let mut result = HashMap::new();
    result.insert(
        HashableValue::String(Rc::new("pid".to_string())),
        Value::Int(pid as i64),
    );
    result.insert(
        HashableValue::String(Rc::new("program".to_string())),
        Value::string(&program),
    );

    // Store the child handle in a thread-safe wrapper for later use
    // For now, we return basic info - the process runs in background
    Ok(Value::Map(Rc::new(std::cell::RefCell::new(result))))
}

fn process_kill(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Process.kill() expects 1 argument, got {}",
            args.len()
        ));
    }

    let pid = get_int_arg(&args[0], "pid")?;
    if pid < 0 {
        return Err("Process.kill() pid must be non-negative".to_string());
    }

    #[cfg(unix)]
    {
        // Use kill command for cross-platform compatibility
        let result = Command::new("kill")
            .arg(pid.to_string())
            .output()
            .map_err(|e| format!("failed to kill process {}: {}", pid, e))?;

        if result.status.success() {
            Ok(Value::Bool(true))
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            Err(format!("failed to kill process {}: {}", pid, stderr.trim()))
        }
    }

    #[cfg(windows)]
    {
        let result = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .map_err(|e| format!("failed to kill process {}: {}", pid, e))?;

        if result.status.success() {
            Ok(Value::Bool(true))
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            Err(format!("failed to kill process {}: {}", pid, stderr.trim()))
        }
    }
}

// ============================================================================
// Signal Module
// ============================================================================

pub fn signal_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "handle" => signal_handle(args),
        _ => Err(format!("Signal has no method '{method}'")),
    }
}

fn signal_handle(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "Signal.handle() expects 2 arguments, got {}",
            args.len()
        ));
    }

    let signal_name = get_string_arg(&args[0], "signal")?;

    // Validate signal name
    let valid_signals = ["SIGINT", "SIGTERM", "SIGHUP", "SIGUSR1", "SIGUSR2"];
    if !valid_signals.contains(&signal_name.as_str()) {
        return Err(format!(
            "Signal.handle() invalid signal '{}'. Valid signals: {}",
            signal_name,
            valid_signals.join(", ")
        ));
    }

    // Validate that second argument is a closure
    match &args[1] {
        Value::Closure(_) => {}
        _ => {
            return Err(format!(
                "Signal.handle() handler must be a closure, got {}",
                args[1].type_name()
            ))
        }
    }

    // Note: Actual signal handling requires VM-level integration.
    // This registers the intent; the VM executor handles the actual signals.
    // For now, return the signal registration info.
    let mut result = HashMap::new();
    result.insert(
        HashableValue::String(Rc::new("signal".to_string())),
        Value::string(&signal_name),
    );
    result.insert(
        HashableValue::String(Rc::new("registered".to_string())),
        Value::Bool(true),
    );

    Ok(Value::Map(Rc::new(std::cell::RefCell::new(result))))
}

// ============================================================================
// Database Module
// ============================================================================

use crate::bytecode::{DbConnection, DbConnectionKind};
use mysql::prelude::Queryable;

/// Db namespace methods (connection factory)
pub fn db_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "sqlite" => db_sqlite(args),
        "postgres" => db_postgres(args),
        "mysql" => db_mysql(args),
        "duckdb" => db_duckdb(args),
        _ => Err(format!("Db has no method '{method}'")),
    }
}

/// Methods on a database connection value
pub fn db_connection_method(
    conn: &Arc<DbConnection>,
    method: &str,
    args: &[Value],
) -> NativeResult {
    match method {
        "query" => db_query(conn, args),
        "execute" => db_execute(conn, args),
        "close" => db_close(conn),
        "begin" => db_begin(conn),
        "commit" => db_commit(conn),
        "rollback" => db_rollback(conn),
        "transaction" => db_transaction(conn, args),
        "prepare" => db_prepare(conn, args),
        "tables" => db_tables(conn),
        "columns" => db_columns(conn, args),
        "table_exists" => db_table_exists(conn, args),
        "version" => Ok(Value::string(&conn.version)),
        "db_type" => Ok(Value::string(conn.db_type())),
        _ => Err(format!("DbConnection has no method '{method}'")),
    }
}

// -----------------------------------------------------------------------------
// Connection Factory Methods
// -----------------------------------------------------------------------------

fn db_sqlite(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 1 {
        return Err(format!(
            "Db.sqlite() expects 1 argument (path), got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;

    let conn = if path == ":memory:" {
        rusqlite::Connection::open_in_memory()
    } else {
        rusqlite::Connection::open(&path)
    };

    let conn = conn.map_err(|e| format!("failed to open SQLite database '{}': {}", path, e))?;
    let db = DbConnection::sqlite(conn, &path)?;
    Ok(Value::DbConnection(Arc::new(db)))
}

fn db_postgres(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Db.postgres() expects 1-2 arguments (url or config), got {}",
            args.len()
        ));
    }

    // Handle either URL string or config map
    let url = match &args[0] {
        Value::String(s) => s.to_string(),
        Value::Map(map) => {
            // Build connection URL from config map
            let map = map.borrow();
            let host = get_map_string(&map, "host").unwrap_or_else(|| "localhost".to_string());
            let port = get_map_int(&map, "port").unwrap_or(5432);
            let user = get_map_string(&map, "user").unwrap_or_else(|| "postgres".to_string());
            let password = get_map_string(&map, "password").unwrap_or_default();
            let database =
                get_map_string(&map, "database").unwrap_or_else(|| "postgres".to_string());
            format!(
                "postgresql://{}:{}@{}:{}/{}",
                user, password, host, port, database
            )
        }
        _ => return Err("Db.postgres() expects a URL string or config map".to_string()),
    };

    let mut client = postgres::Client::connect(&url, postgres::NoTls)
        .map_err(|e| format!("failed to connect to PostgreSQL: {}", e))?;

    // Get version
    let version: String = client
        .query_one("SELECT version()", &[])
        .map_err(|e| format!("failed to get PostgreSQL version: {}", e))?
        .get(0);

    let mut db = DbConnection::postgres(client)?;
    db.version = version;
    db.identifier = url;
    Ok(Value::DbConnection(Arc::new(db)))
}

fn db_mysql(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Db.mysql() expects 1-2 arguments (url or config), got {}",
            args.len()
        ));
    }

    // Handle either URL string or config map
    let url = match &args[0] {
        Value::String(s) => s.to_string(),
        Value::Map(map) => {
            // Build connection URL from config map
            let map = map.borrow();
            let host = get_map_string(&map, "host").unwrap_or_else(|| "localhost".to_string());
            let port = get_map_int(&map, "port").unwrap_or(3306);
            let user = get_map_string(&map, "user").unwrap_or_else(|| "root".to_string());
            let password = get_map_string(&map, "password").unwrap_or_default();
            let database = get_map_string(&map, "database").unwrap_or_default();
            if database.is_empty() {
                format!("mysql://{}:{}@{}:{}", user, password, host, port)
            } else {
                format!(
                    "mysql://{}:{}@{}:{}/{}",
                    user, password, host, port, database
                )
            }
        }
        _ => return Err("Db.mysql() expects a URL string or config map".to_string()),
    };

    let opts = mysql::Opts::from_url(&url).map_err(|e| format!("invalid MySQL URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| format!("failed to connect to MySQL: {}", e))?;

    // Get version
    let version: String = conn
        .query_first("SELECT VERSION()")
        .map_err(|e| format!("failed to get MySQL version: {}", e))?
        .unwrap_or_else(|| "unknown".to_string());

    let mut db = DbConnection::mysql(conn, &url)?;
    db.version = format!("MySQL {}", version);
    Ok(Value::DbConnection(Arc::new(db)))
}

fn db_duckdb(args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 1 {
        return Err(format!(
            "Db.duckdb() expects 1 argument (path), got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;

    let conn = if path == ":memory:" {
        duckdb::Connection::open_in_memory()
    } else {
        duckdb::Connection::open(&path)
    };

    let conn = conn.map_err(|e| format!("failed to open DuckDB database '{}': {}", path, e))?;
    let db = DbConnection::duckdb(conn, &path)?;
    Ok(Value::DbConnection(Arc::new(db)))
}

// -----------------------------------------------------------------------------
// Helper Functions for Map Access
// -----------------------------------------------------------------------------

fn get_map_string(map: &HashMap<HashableValue, Value>, key: &str) -> Option<String> {
    let key = HashableValue::String(Rc::new(key.to_string()));
    match map.get(&key) {
        Some(Value::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

fn get_map_int(map: &HashMap<HashableValue, Value>, key: &str) -> Option<i64> {
    let key = HashableValue::String(Rc::new(key.to_string()));
    match map.get(&key) {
        Some(Value::Int(i)) => Some(*i),
        _ => None,
    }
}

// -----------------------------------------------------------------------------
// Connection Methods
// -----------------------------------------------------------------------------

fn db_query(conn: &Arc<DbConnection>, args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "query() expects 1-2 arguments (sql, params?), got {}",
            args.len()
        ));
    }
    let sql = get_string_arg(&args[0], "sql")?;
    let params = if args.len() > 1 {
        extract_params(&args[1])?
    } else {
        Vec::new()
    };

    match &conn.kind {
        DbConnectionKind::Sqlite(c) => sqlite_query(c, &sql, &params),
        DbConnectionKind::Postgres(c) => postgres_query(c, &sql, &params),
        DbConnectionKind::MySql(c) => mysql_query(c, &sql, &params),
        DbConnectionKind::DuckDb(c) => duckdb_query(c, &sql, &params),
    }
}

fn db_execute(conn: &Arc<DbConnection>, args: &[Value]) -> NativeResult {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "execute() expects 1-2 arguments (sql, params?), got {}",
            args.len()
        ));
    }
    let sql = get_string_arg(&args[0], "sql")?;
    let params = if args.len() > 1 {
        extract_params(&args[1])?
    } else {
        Vec::new()
    };

    match &conn.kind {
        DbConnectionKind::Sqlite(c) => sqlite_execute(c, &sql, &params),
        DbConnectionKind::Postgres(c) => postgres_execute(c, &sql, &params),
        DbConnectionKind::MySql(c) => mysql_execute(c, &sql, &params),
        DbConnectionKind::DuckDb(c) => duckdb_execute(c, &sql, &params),
    }
}

fn db_close(_conn: &Arc<DbConnection>) -> NativeResult {
    // Connections are automatically closed when Arc reference count drops to 0
    // This is just a hint that the user wants to close early
    Ok(Value::Null)
}

fn db_begin(conn: &Arc<DbConnection>) -> NativeResult {
    match &conn.kind {
        DbConnectionKind::Sqlite(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            c.execute("BEGIN TRANSACTION", [])
                .map_err(|e| format!("failed to begin transaction: {}", e))?;
        }
        DbConnectionKind::Postgres(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            c.execute("BEGIN", &[])
                .map_err(|e| format!("failed to begin transaction: {}", e))?;
        }
        DbConnectionKind::MySql(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            c.query_drop("START TRANSACTION")
                .map_err(|e| format!("failed to begin transaction: {}", e))?;
        }
        DbConnectionKind::DuckDb(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            c.execute("BEGIN TRANSACTION", [])
                .map_err(|e| format!("failed to begin transaction: {}", e))?;
        }
    }
    Ok(Value::Null)
}

fn db_commit(conn: &Arc<DbConnection>) -> NativeResult {
    match &conn.kind {
        DbConnectionKind::Sqlite(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            c.execute("COMMIT", [])
                .map_err(|e| format!("failed to commit transaction: {}", e))?;
        }
        DbConnectionKind::Postgres(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            c.execute("COMMIT", &[])
                .map_err(|e| format!("failed to commit transaction: {}", e))?;
        }
        DbConnectionKind::MySql(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            c.query_drop("COMMIT")
                .map_err(|e| format!("failed to commit transaction: {}", e))?;
        }
        DbConnectionKind::DuckDb(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            c.execute("COMMIT", [])
                .map_err(|e| format!("failed to commit transaction: {}", e))?;
        }
    }
    Ok(Value::Null)
}

fn db_rollback(conn: &Arc<DbConnection>) -> NativeResult {
    match &conn.kind {
        DbConnectionKind::Sqlite(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            c.execute("ROLLBACK", [])
                .map_err(|e| format!("failed to rollback transaction: {}", e))?;
        }
        DbConnectionKind::Postgres(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            c.execute("ROLLBACK", &[])
                .map_err(|e| format!("failed to rollback transaction: {}", e))?;
        }
        DbConnectionKind::MySql(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            c.query_drop("ROLLBACK")
                .map_err(|e| format!("failed to rollback transaction: {}", e))?;
        }
        DbConnectionKind::DuckDb(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            c.execute("ROLLBACK", [])
                .map_err(|e| format!("failed to rollback transaction: {}", e))?;
        }
    }
    Ok(Value::Null)
}

fn db_transaction(_conn: &Arc<DbConnection>, _args: &[Value]) -> NativeResult {
    // Transaction with callback requires closure execution from VM
    // This would need special handling - defer for now
    Err("transaction() with callback is not yet supported. Use begin()/commit()/rollback() instead.".to_string())
}

fn db_prepare(_conn: &Arc<DbConnection>, _args: &[Value]) -> NativeResult {
    // Prepared statements would need a new Value variant
    // Defer for now - the main query/execute already support parameters
    Err("prepared statements are not yet supported. Use query() or execute() with parameters instead.".to_string())
}

// -----------------------------------------------------------------------------
// Metadata Methods
// -----------------------------------------------------------------------------

fn db_tables(conn: &Arc<DbConnection>) -> NativeResult {
    let tables = match &conn.kind {
        DbConnectionKind::Sqlite(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            let mut stmt = c
                .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
                .map_err(|e| format!("failed to list tables: {}", e))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| format!("failed to list tables: {}", e))?;
            rows.filter_map(Result::ok).map(Value::string).collect()
        }
        DbConnectionKind::Postgres(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            let rows = c
                .query("SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename", &[])
                .map_err(|e| format!("failed to list tables: {}", e))?;
            rows.iter()
                .map(|row| Value::string(row.get::<_, String>(0)))
                .collect()
        }
        DbConnectionKind::MySql(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            let rows: Vec<String> = c
                .query("SHOW TABLES")
                .map_err(|e| format!("failed to list tables: {}", e))?;
            rows.into_iter().map(Value::string).collect()
        }
        DbConnectionKind::DuckDb(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            let mut stmt = c
                .prepare("SELECT table_name FROM information_schema.tables WHERE table_schema = 'main' ORDER BY table_name")
                .map_err(|e| format!("failed to list tables: {}", e))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| format!("failed to list tables: {}", e))?;
            rows.filter_map(Result::ok).map(Value::string).collect()
        }
    };
    Ok(Value::list(tables))
}

fn db_columns(conn: &Arc<DbConnection>, args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "columns() expects 1 argument (table_name), got {}",
            args.len()
        ));
    }
    let table = get_string_arg(&args[0], "table")?;

    let columns = match &conn.kind {
        DbConnectionKind::Sqlite(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            let mut stmt = c
                .prepare(&format!("PRAGMA table_info('{}')", table))
                .map_err(|e| format!("failed to get columns: {}", e))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(1)?,   // name
                        row.get::<_, String>(2)?,   // type
                        row.get::<_, i32>(3)? == 0, // nullable (notnull=0 means nullable)
                        row.get::<_, i32>(5)? == 1, // primary_key
                    ))
                })
                .map_err(|e| format!("failed to get columns: {}", e))?;
            rows.filter_map(Result::ok)
                .map(|(name, type_, nullable, pk)| column_to_map(name, type_, nullable, pk))
                .collect()
        }
        DbConnectionKind::Postgres(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            let sql = "SELECT column_name, data_type, is_nullable,
                       EXISTS(SELECT 1 FROM information_schema.table_constraints tc
                              JOIN information_schema.key_column_usage kcu
                              ON tc.constraint_name = kcu.constraint_name
                              WHERE tc.constraint_type = 'PRIMARY KEY'
                              AND tc.table_name = $1
                              AND kcu.column_name = columns.column_name) as is_pk
                       FROM information_schema.columns
                       WHERE table_name = $1
                       ORDER BY ordinal_position";
            let rows = c
                .query(sql, &[&table])
                .map_err(|e| format!("failed to get columns: {}", e))?;
            rows.iter()
                .map(|row| {
                    let name: String = row.get(0);
                    let type_: String = row.get(1);
                    let nullable: String = row.get(2);
                    let pk: bool = row.get(3);
                    column_to_map(name, type_, nullable == "YES", pk)
                })
                .collect()
        }
        DbConnectionKind::MySql(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            let rows: Vec<(String, String, String, String)> = c
                .exec(
                    "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_KEY FROM information_schema.columns WHERE table_name = ?",
                    (&table,),
                )
                .map_err(|e| format!("failed to get columns: {}", e))?;
            rows.into_iter()
                .map(|(name, type_, nullable, key)| {
                    column_to_map(name, type_, nullable == "YES", key == "PRI")
                })
                .collect()
        }
        DbConnectionKind::DuckDb(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            let sql = format!(
                "SELECT column_name, data_type, is_nullable,
                 COALESCE((SELECT true FROM duckdb_constraints()
                           WHERE table_name = '{}'
                           AND constraint_type = 'PRIMARY KEY'
                           AND constraint_column_names @> ARRAY[columns.column_name]), false)
                 FROM information_schema.columns
                 WHERE table_name = '{}'
                 ORDER BY ordinal_position",
                table, table
            );
            let mut stmt = c
                .prepare(&sql)
                .map_err(|e| format!("failed to get columns: {}", e))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, bool>(3).unwrap_or(false),
                    ))
                })
                .map_err(|e| format!("failed to get columns: {}", e))?;
            rows.filter_map(Result::ok)
                .map(|(name, type_, nullable, pk)| {
                    column_to_map(name, type_, nullable == "YES", pk)
                })
                .collect()
        }
    };
    Ok(Value::list(columns))
}

fn column_to_map(name: String, type_: String, nullable: bool, primary_key: bool) -> Value {
    let mut map = HashMap::new();
    map.insert(
        HashableValue::String(Rc::new("name".to_string())),
        Value::string(name),
    );
    map.insert(
        HashableValue::String(Rc::new("type".to_string())),
        Value::string(type_),
    );
    map.insert(
        HashableValue::String(Rc::new("nullable".to_string())),
        Value::Bool(nullable),
    );
    map.insert(
        HashableValue::String(Rc::new("primary_key".to_string())),
        Value::Bool(primary_key),
    );
    Value::Map(Rc::new(RefCell::new(map)))
}

fn db_table_exists(conn: &Arc<DbConnection>, args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "table_exists() expects 1 argument (table_name), got {}",
            args.len()
        ));
    }
    let table = get_string_arg(&args[0], "table")?;

    let exists = match &conn.kind {
        DbConnectionKind::Sqlite(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            let count: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?",
                    [&table],
                    |row| row.get(0),
                )
                .map_err(|e| format!("failed to check table: {}", e))?;
            count > 0
        }
        DbConnectionKind::Postgres(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            let row = c
                .query_one(
                    "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = $1)",
                    &[&table],
                )
                .map_err(|e| format!("failed to check table: {}", e))?;
            row.get(0)
        }
        DbConnectionKind::MySql(c) => {
            let mut c = c.lock().map_err(|_| "failed to lock connection")?;
            let result: Option<String> = c
                .exec_first("SHOW TABLES LIKE ?", (&table,))
                .map_err(|e| format!("failed to check table: {}", e))?;
            result.is_some()
        }
        DbConnectionKind::DuckDb(c) => {
            let c = c.lock().map_err(|_| "failed to lock connection")?;
            let count: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = ?",
                    [&table],
                    |row| row.get(0),
                )
                .map_err(|e| format!("failed to check table: {}", e))?;
            count > 0
        }
    };
    Ok(Value::Bool(exists))
}

// -----------------------------------------------------------------------------
// Parameter Extraction
// -----------------------------------------------------------------------------

fn extract_params(value: &Value) -> Result<Vec<DbParam>, String> {
    match value {
        Value::List(list) => list.borrow().iter().map(value_to_param).collect(),
        _ => Err("parameters must be a List".to_string()),
    }
}

#[derive(Debug, Clone)]
enum DbParam {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

fn value_to_param(value: &Value) -> Result<DbParam, String> {
    match value {
        Value::Null => Ok(DbParam::Null),
        Value::Bool(b) => Ok(DbParam::Bool(*b)),
        Value::Int(i) => Ok(DbParam::Int(*i)),
        Value::Float(f) => Ok(DbParam::Float(*f)),
        Value::String(s) => Ok(DbParam::String(s.to_string())),
        _ => Err(format!("unsupported parameter type: {}", value.type_name())),
    }
}

// -----------------------------------------------------------------------------
// SQLite Implementation
// -----------------------------------------------------------------------------

fn sqlite_query(
    conn: &std::sync::Mutex<rusqlite::Connection>,
    sql: &str,
    params: &[DbParam],
) -> NativeResult {
    let conn = conn.lock().map_err(|_| "failed to lock connection")?;

    let mut stmt = conn.prepare(sql).map_err(|e| format!("SQL error: {}", e))?;

    // Get column names
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    // Convert params
    let sqlite_params: Vec<rusqlite::types::Value> = params
        .iter()
        .map(|p| match p {
            DbParam::Null => rusqlite::types::Value::Null,
            DbParam::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
            DbParam::Int(i) => rusqlite::types::Value::Integer(*i),
            DbParam::Float(f) => rusqlite::types::Value::Real(*f),
            DbParam::String(s) => rusqlite::types::Value::Text(s.clone()),
        })
        .collect();

    let param_refs: Vec<&dyn rusqlite::ToSql> = sqlite_params
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let mut map = HashMap::new();
            for (i, name) in column_names.iter().enumerate() {
                let value = sqlite_value_to_stratum(row.get_ref(i)?);
                map.insert(HashableValue::String(Rc::new(name.clone())), value);
            }
            Ok(Value::Map(Rc::new(RefCell::new(map))))
        })
        .map_err(|e| format!("query error: {}", e))?;

    let results: Vec<Value> = rows.filter_map(Result::ok).collect();

    Ok(Value::list(results))
}

fn sqlite_execute(
    conn: &std::sync::Mutex<rusqlite::Connection>,
    sql: &str,
    params: &[DbParam],
) -> NativeResult {
    let conn = conn.lock().map_err(|_| "failed to lock connection")?;

    // Convert params
    let sqlite_params: Vec<rusqlite::types::Value> = params
        .iter()
        .map(|p| match p {
            DbParam::Null => rusqlite::types::Value::Null,
            DbParam::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
            DbParam::Int(i) => rusqlite::types::Value::Integer(*i),
            DbParam::Float(f) => rusqlite::types::Value::Real(*f),
            DbParam::String(s) => rusqlite::types::Value::Text(s.clone()),
        })
        .collect();

    let param_refs: Vec<&dyn rusqlite::ToSql> = sqlite_params
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

    let count = conn
        .execute(sql, param_refs.as_slice())
        .map_err(|e| format!("execute error: {}", e))?;

    Ok(Value::Int(count as i64))
}

fn sqlite_value_to_stratum(value: rusqlite::types::ValueRef<'_>) -> Value {
    use rusqlite::types::ValueRef;
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::Int(i),
        ValueRef::Real(f) => Value::Float(f),
        ValueRef::Text(s) => Value::string(String::from_utf8_lossy(s)),
        ValueRef::Blob(b) => {
            // Convert blob to list of bytes
            Value::list(b.iter().map(|&byte| Value::Int(byte as i64)).collect())
        }
    }
}

// -----------------------------------------------------------------------------
// PostgreSQL Implementation
// -----------------------------------------------------------------------------

fn postgres_query(
    conn: &std::sync::Mutex<postgres::Client>,
    sql: &str,
    params: &[DbParam],
) -> NativeResult {
    let mut conn = conn.lock().map_err(|_| "failed to lock connection")?;

    // Convert params to postgres types
    let pg_params: Vec<Box<dyn postgres::types::ToSql + Sync + Send>> = params
        .iter()
        .map(|p| -> Box<dyn postgres::types::ToSql + Sync + Send> {
            match p {
                DbParam::Null => Box::new(Option::<String>::None),
                DbParam::Bool(b) => Box::new(*b),
                DbParam::Int(i) => Box::new(*i),
                DbParam::Float(f) => Box::new(*f),
                DbParam::String(s) => Box::new(s.clone()),
            }
        })
        .collect();

    let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = pg_params
        .iter()
        .map(|p| p.as_ref() as &(dyn postgres::types::ToSql + Sync))
        .collect();

    let rows = conn
        .query(sql, &param_refs)
        .map_err(|e| format!("query error: {}", e))?;

    let results: Vec<Value> = rows
        .iter()
        .map(|row| postgres_row_to_stratum(row))
        .collect();

    Ok(Value::list(results))
}

fn postgres_execute(
    conn: &std::sync::Mutex<postgres::Client>,
    sql: &str,
    params: &[DbParam],
) -> NativeResult {
    let mut conn = conn.lock().map_err(|_| "failed to lock connection")?;

    // Convert params to postgres types
    let pg_params: Vec<Box<dyn postgres::types::ToSql + Sync + Send>> = params
        .iter()
        .map(|p| -> Box<dyn postgres::types::ToSql + Sync + Send> {
            match p {
                DbParam::Null => Box::new(Option::<String>::None),
                DbParam::Bool(b) => Box::new(*b),
                DbParam::Int(i) => Box::new(*i),
                DbParam::Float(f) => Box::new(*f),
                DbParam::String(s) => Box::new(s.clone()),
            }
        })
        .collect();

    let param_refs: Vec<&(dyn postgres::types::ToSql + Sync)> = pg_params
        .iter()
        .map(|p| p.as_ref() as &(dyn postgres::types::ToSql + Sync))
        .collect();

    let count = conn
        .execute(sql, &param_refs)
        .map_err(|e| format!("execute error: {}", e))?;

    Ok(Value::Int(count as i64))
}

fn postgres_row_to_stratum(row: &postgres::Row) -> Value {
    let mut map = HashMap::new();

    for (i, column) in row.columns().iter().enumerate() {
        let name = column.name().to_string();
        let value = postgres_column_to_stratum(row, i, column.type_());
        map.insert(HashableValue::String(Rc::new(name)), value);
    }

    Value::Map(Rc::new(RefCell::new(map)))
}

fn postgres_column_to_stratum(
    row: &postgres::Row,
    idx: usize,
    type_: &postgres::types::Type,
) -> Value {
    use postgres::types::Type;

    // Try to get as the appropriate type based on the column type
    match *type_ {
        Type::BOOL => row
            .get::<_, Option<bool>>(idx)
            .map_or(Value::Null, Value::Bool),
        Type::INT2 => row
            .get::<_, Option<i16>>(idx)
            .map_or(Value::Null, |v| Value::Int(v as i64)),
        Type::INT4 => row
            .get::<_, Option<i32>>(idx)
            .map_or(Value::Null, |v| Value::Int(v as i64)),
        Type::INT8 => row
            .get::<_, Option<i64>>(idx)
            .map_or(Value::Null, Value::Int),
        Type::FLOAT4 => row
            .get::<_, Option<f32>>(idx)
            .map_or(Value::Null, |v| Value::Float(v as f64)),
        Type::FLOAT8 => row
            .get::<_, Option<f64>>(idx)
            .map_or(Value::Null, Value::Float),
        Type::TEXT | Type::VARCHAR | Type::CHAR | Type::NAME => row
            .get::<_, Option<String>>(idx)
            .map_or(Value::Null, Value::string),
        _ => {
            // Try as string for unknown types
            row.get::<_, Option<String>>(idx)
                .map_or(Value::Null, Value::string)
        }
    }
}

// -----------------------------------------------------------------------------
// MySQL Implementation
// -----------------------------------------------------------------------------

fn mysql_query(
    conn: &std::sync::Mutex<mysql::Conn>,
    sql: &str,
    params: &[DbParam],
) -> NativeResult {
    let mut conn = conn.lock().map_err(|_| "failed to lock connection")?;

    // Convert params to mysql types
    let mysql_params: Vec<mysql::Value> = params
        .iter()
        .map(|p| match p {
            DbParam::Null => mysql::Value::NULL,
            DbParam::Bool(b) => mysql::Value::from(*b),
            DbParam::Int(i) => mysql::Value::from(*i),
            DbParam::Float(f) => mysql::Value::from(*f),
            DbParam::String(s) => mysql::Value::from(s.clone()),
        })
        .collect();

    let rows: Vec<mysql::Row> = conn
        .exec(sql, mysql::Params::Positional(mysql_params))
        .map_err(|e| format!("query error: {}", e))?;

    let results: Vec<Value> = rows.iter().map(mysql_row_to_stratum).collect();

    Ok(Value::list(results))
}

fn mysql_execute(
    conn: &std::sync::Mutex<mysql::Conn>,
    sql: &str,
    params: &[DbParam],
) -> NativeResult {
    let mut conn = conn.lock().map_err(|_| "failed to lock connection")?;

    // Convert params to mysql types
    let mysql_params: Vec<mysql::Value> = params
        .iter()
        .map(|p| match p {
            DbParam::Null => mysql::Value::NULL,
            DbParam::Bool(b) => mysql::Value::from(*b),
            DbParam::Int(i) => mysql::Value::from(*i),
            DbParam::Float(f) => mysql::Value::from(*f),
            DbParam::String(s) => mysql::Value::from(s.clone()),
        })
        .collect();

    conn.exec_drop(sql, mysql::Params::Positional(mysql_params))
        .map_err(|e| format!("execute error: {}", e))?;

    Ok(Value::Int(conn.affected_rows() as i64))
}

fn mysql_row_to_stratum(row: &mysql::Row) -> Value {
    let mut map = HashMap::new();

    for (i, column) in row.columns_ref().iter().enumerate() {
        let name = column.name_str().to_string();
        let value =
            mysql_value_to_stratum(row.get::<mysql::Value, _>(i).unwrap_or(mysql::Value::NULL));
        map.insert(HashableValue::String(Rc::new(name)), value);
    }

    Value::Map(Rc::new(RefCell::new(map)))
}

fn mysql_value_to_stratum(value: mysql::Value) -> Value {
    match value {
        mysql::Value::NULL => Value::Null,
        mysql::Value::Bytes(bytes) => {
            // Try to convert to string, fall back to byte list
            match String::from_utf8(bytes.clone()) {
                Ok(s) => Value::string(s),
                Err(_) => Value::list(bytes.iter().map(|&b| Value::Int(b as i64)).collect()),
            }
        }
        mysql::Value::Int(i) => Value::Int(i),
        mysql::Value::UInt(u) => Value::Int(u as i64),
        mysql::Value::Float(f) => Value::Float(f as f64),
        mysql::Value::Double(d) => Value::Float(d),
        mysql::Value::Date(y, m, d, h, mi, s, _us) => Value::string(format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            y, m, d, h, mi, s
        )),
        mysql::Value::Time(neg, d, h, mi, s, _us) => {
            let sign = if neg { "-" } else { "" };
            let total_hours = d * 24 + u32::from(h);
            Value::string(format!("{}{}:{:02}:{:02}", sign, total_hours, mi, s))
        }
    }
}

// -----------------------------------------------------------------------------
// DuckDB Implementation
// -----------------------------------------------------------------------------

fn duckdb_query(
    conn: &std::sync::Mutex<duckdb::Connection>,
    sql: &str,
    params: &[DbParam],
) -> NativeResult {
    let conn = conn.lock().map_err(|_| "failed to lock connection")?;

    let mut stmt = conn.prepare(sql).map_err(|e| format!("SQL error: {}", e))?;

    // Convert params
    let duckdb_params: Vec<duckdb::types::Value> = params
        .iter()
        .map(|p| match p {
            DbParam::Null => duckdb::types::Value::Null,
            DbParam::Bool(b) => duckdb::types::Value::Boolean(*b),
            DbParam::Int(i) => duckdb::types::Value::BigInt(*i),
            DbParam::Float(f) => duckdb::types::Value::Double(*f),
            DbParam::String(s) => duckdb::types::Value::Text(s.clone()),
        })
        .collect();

    let param_refs: Vec<&dyn duckdb::ToSql> = duckdb_params
        .iter()
        .map(|v| v as &dyn duckdb::ToSql)
        .collect();

    // Use query_map which handles the iteration and provides column access
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            // Get column count from statement reference in the rows
            let stmt_ref = row.as_ref();
            let col_count = stmt_ref.column_count();
            let mut map: HashMap<HashableValue, Value> = HashMap::new();
            for i in 0..col_count {
                let name = stmt_ref
                    .column_name(i)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| format!("col{}", i));
                let value = duckdb_value_to_stratum(
                    row.get_ref(i).unwrap_or(duckdb::types::ValueRef::Null),
                );
                map.insert(HashableValue::String(Rc::new(name)), value);
            }
            Ok(Value::Map(Rc::new(RefCell::new(map))))
        })
        .map_err(|e| format!("query error: {}", e))?;

    let results: Vec<Value> = rows.filter_map(Result::ok).collect();
    Ok(Value::list(results))
}

fn duckdb_execute(
    conn: &std::sync::Mutex<duckdb::Connection>,
    sql: &str,
    params: &[DbParam],
) -> NativeResult {
    let conn = conn.lock().map_err(|_| "failed to lock connection")?;

    // Convert params
    let duckdb_params: Vec<duckdb::types::Value> = params
        .iter()
        .map(|p| match p {
            DbParam::Null => duckdb::types::Value::Null,
            DbParam::Bool(b) => duckdb::types::Value::Boolean(*b),
            DbParam::Int(i) => duckdb::types::Value::BigInt(*i),
            DbParam::Float(f) => duckdb::types::Value::Double(*f),
            DbParam::String(s) => duckdb::types::Value::Text(s.clone()),
        })
        .collect();

    let param_refs: Vec<&dyn duckdb::ToSql> = duckdb_params
        .iter()
        .map(|v| v as &dyn duckdb::ToSql)
        .collect();

    let count = conn
        .execute(sql, param_refs.as_slice())
        .map_err(|e| format!("execute error: {}", e))?;

    Ok(Value::Int(count as i64))
}

fn duckdb_value_to_stratum(value: duckdb::types::ValueRef<'_>) -> Value {
    use duckdb::types::ValueRef;
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Boolean(b) => Value::Bool(b),
        ValueRef::TinyInt(i) => Value::Int(i as i64),
        ValueRef::SmallInt(i) => Value::Int(i as i64),
        ValueRef::Int(i) => Value::Int(i as i64),
        ValueRef::BigInt(i) => Value::Int(i),
        ValueRef::HugeInt(i) => Value::Int(i as i64), // May lose precision for very large values
        ValueRef::UTinyInt(i) => Value::Int(i as i64),
        ValueRef::USmallInt(i) => Value::Int(i as i64),
        ValueRef::UInt(i) => Value::Int(i as i64),
        ValueRef::UBigInt(i) => Value::Int(i as i64),
        ValueRef::Float(f) => Value::Float(f as f64),
        ValueRef::Double(f) => Value::Float(f),
        ValueRef::Text(s) => Value::string(String::from_utf8_lossy(s)),
        ValueRef::Blob(b) => Value::list(b.iter().map(|&byte| Value::Int(byte as i64)).collect()),
        _ => Value::Null, // For unsupported types like Date, Time, Timestamp, etc.
    }
}

// ============================================================================
// Async Module - Async utilities (sleep, all, race, timeout)
// ============================================================================

pub fn async_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "sleep" => async_sleep(args),
        "ready" => async_ready(args),
        "failed" => async_failed(args),
        "all" => async_all(args),
        "race" => async_race(args),
        "timeout" => async_timeout(args),
        "spawn" => async_spawn(args),
        _ => Err(format!("Async has no method '{method}'")),
    }
}

/// Create a pending future that represents an async sleep
/// In a real async execution, the executor would wait for the specified duration
/// The returned Future starts as Pending and needs to be resolved by the executor
fn async_sleep(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("sleep requires a duration argument (ms as Int)".to_string());
    }
    let ms = match &args[0] {
        Value::Int(n) => *n,
        _ => return Err("sleep requires an Int (milliseconds)".to_string()),
    };

    if ms < 0 {
        return Err("sleep duration cannot be negative".to_string());
    }

    // Create a pending future
    // The executor will detect this is a sleep future and wait accordingly
    // For now, we store the duration info in the result field as metadata
    let future = FutureState::pending_with_metadata(Value::Int(ms), "sleep".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Create a future that is immediately ready with a value
fn async_ready(args: &[Value]) -> NativeResult {
    let value = if args.is_empty() {
        Value::Null
    } else {
        args[0].clone()
    };

    let future = FutureState::ready(value);
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Create a future that is immediately failed with an error
fn async_failed(args: &[Value]) -> NativeResult {
    let msg = if args.is_empty() {
        "unknown error".to_string()
    } else {
        match &args[0] {
            Value::String(s) => (**s).clone(),
            v => v.to_string(),
        }
    };

    let future = FutureState::failed(msg);
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Async.all(futures) - Wait for all futures to complete
/// Returns a Future<List<T>> containing all results in order
fn async_all(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("Async.all() requires a List of futures".to_string());
    }

    let futures = match &args[0] {
        Value::List(list) => {
            let items = list.borrow();
            // Validate all items are futures
            for (i, item) in items.iter().enumerate() {
                if !matches!(item, Value::Future(_)) {
                    return Err(format!(
                        "Async.all() element at index {i} is not a Future, got {}",
                        item.type_name()
                    ));
                }
            }
            Value::List(Rc::new(RefCell::new(items.clone())))
        }
        _ => {
            return Err(format!(
                "Async.all() expects List<Future>, got {}",
                args[0].type_name()
            ))
        }
    };

    let future = FutureState::pending_with_metadata(futures, "all".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Async.race(futures) - Wait for the first future to complete
/// Returns the result of the first future that completes
fn async_race(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("Async.race() requires a List of futures".to_string());
    }

    let futures = match &args[0] {
        Value::List(list) => {
            let items = list.borrow();
            if items.is_empty() {
                return Err("Async.race() requires at least one future".to_string());
            }
            // Validate all items are futures
            for (i, item) in items.iter().enumerate() {
                if !matches!(item, Value::Future(_)) {
                    return Err(format!(
                        "Async.race() element at index {i} is not a Future, got {}",
                        item.type_name()
                    ));
                }
            }
            Value::List(Rc::new(RefCell::new(items.clone())))
        }
        _ => {
            return Err(format!(
                "Async.race() expects List<Future>, got {}",
                args[0].type_name()
            ))
        }
    };

    let future = FutureState::pending_with_metadata(futures, "race".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Async.timeout(future, ms) - Add a timeout to a future
/// Returns the future's result if it completes in time, or fails with timeout error
fn async_timeout(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err("Async.timeout() requires (future, ms)".to_string());
    }

    let inner_future = match &args[0] {
        Value::Future(_) => args[0].clone(),
        _ => {
            return Err(format!(
                "Async.timeout() first argument must be Future, got {}",
                args[0].type_name()
            ))
        }
    };

    let timeout_ms = match &args[1] {
        Value::Int(ms) if *ms >= 0 => *ms,
        Value::Int(ms) => return Err(format!("Async.timeout() ms must be non-negative, got {ms}")),
        _ => {
            return Err(format!(
                "Async.timeout() second argument must be Int (ms), got {}",
                args[1].type_name()
            ))
        }
    };

    // Store both the future and timeout in a map
    let mut metadata_map = std::collections::HashMap::new();
    metadata_map.insert(
        HashableValue::String(Rc::new("future".to_string())),
        inner_future,
    );
    metadata_map.insert(
        HashableValue::String(Rc::new("ms".to_string())),
        Value::Int(timeout_ms),
    );
    let metadata = Value::Map(Rc::new(RefCell::new(metadata_map)));

    let future = FutureState::pending_with_metadata(metadata, "timeout".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Async.spawn(closure) - Spawn a closure on a parallel OS thread
/// Returns a Future<T> that resolves when the thread completes
fn async_spawn(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("Async.spawn() requires a closure argument".to_string());
    }

    let closure = match &args[0] {
        Value::Closure(_) => args[0].clone(),
        _ => {
            return Err(format!(
                "Async.spawn() expects a closure, got {}",
                args[0].type_name()
            ))
        }
    };

    let future = FutureState::pending_with_metadata(closure, "spawn".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

// ============================================================================
// TCP Module - TCP networking (client and server)
// ============================================================================

pub fn tcp_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "connect" => tcp_connect(args),
        "listen" => tcp_listen(args),
        _ => Err(format!("Tcp has no method '{method}'")),
    }
}

/// Tcp.connect(host, port) - Create a pending future that connects to a TCP server
/// Returns a Future<TcpStream>
fn tcp_connect(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "Tcp.connect() expects 2 arguments (host, port), got {}",
            args.len()
        ));
    }

    let host = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => {
            return Err(format!(
                "Tcp.connect() host must be String, got {}",
                args[0].type_name()
            ))
        }
    };

    let port = match &args[1] {
        Value::Int(p) if *p > 0 && *p <= 65535 => *p as u16,
        Value::Int(p) => return Err(format!("Tcp.connect() port must be 1-65535, got {p}")),
        _ => {
            return Err(format!(
                "Tcp.connect() port must be Int, got {}",
                args[1].type_name()
            ))
        }
    };

    // Store host:port as metadata for the executor to use
    let metadata = Value::string(format!("{host}:{port}"));
    let future = FutureState::pending_with_metadata(metadata, "tcp_connect".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Tcp.listen(addr, port) - Create a pending future that binds a TCP listener
/// Returns a Future<TcpListener>
fn tcp_listen(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "Tcp.listen() expects 2 arguments (addr, port), got {}",
            args.len()
        ));
    }

    let addr = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => {
            return Err(format!(
                "Tcp.listen() addr must be String, got {}",
                args[0].type_name()
            ))
        }
    };

    let port = match &args[1] {
        Value::Int(p) if *p >= 0 && *p <= 65535 => *p as u16,
        Value::Int(p) => return Err(format!("Tcp.listen() port must be 0-65535, got {p}")),
        _ => {
            return Err(format!(
                "Tcp.listen() port must be Int, got {}",
                args[1].type_name()
            ))
        }
    };

    // Store addr:port as metadata for the executor to use
    let metadata = Value::string(format!("{addr}:{port}"));
    let future = FutureState::pending_with_metadata(metadata, "tcp_listen".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Methods on TcpStream value type
pub fn tcp_stream_method(
    stream: &Arc<TcpStreamWrapper>,
    method: &str,
    args: &[Value],
) -> NativeResult {
    match method {
        "read" => tcp_stream_read(stream, args),
        "read_exact" => tcp_stream_read_exact(stream, args),
        "write" => tcp_stream_write(stream, args),
        "close" | "shutdown" => tcp_stream_close(stream),
        "peer_addr" => Ok(Value::string(&stream.peer_addr)),
        "local_addr" => Ok(Value::string(&stream.local_addr)),
        _ => Err(format!("TcpStream has no method '{method}'")),
    }
}

/// stream.read(max_bytes?) - Read up to max_bytes from the stream (async)
fn tcp_stream_read(stream: &Arc<TcpStreamWrapper>, args: &[Value]) -> NativeResult {
    let max_bytes = if args.is_empty() {
        8192 // Default buffer size
    } else {
        match &args[0] {
            Value::Int(n) if *n > 0 => *n as usize,
            Value::Int(n) => return Err(format!("read max_bytes must be positive, got {n}")),
            _ => {
                return Err(format!(
                    "read max_bytes must be Int, got {}",
                    args[0].type_name()
                ))
            }
        }
    };

    // Create metadata with stream reference and buffer size
    let metadata = Value::Map(Rc::new(RefCell::new({
        let mut m = HashMap::new();
        m.insert(
            HashableValue::String(Rc::new("stream_addr".into())),
            Value::string(&stream.local_addr),
        );
        m.insert(
            HashableValue::String(Rc::new("peer_addr".into())),
            Value::string(&stream.peer_addr),
        );
        m.insert(
            HashableValue::String(Rc::new("max_bytes".into())),
            Value::Int(max_bytes as i64),
        );
        m
    })));

    // Store the actual stream Arc in a static map keyed by address for executor to retrieve
    // For now, we use a simpler approach: create a pending future with metadata
    let future = FutureState::pending_with_metadata(metadata, "tcp_read".to_string());
    let future_ref = Rc::new(RefCell::new(future));

    // Store the stream reference in the future's metadata for the executor
    // The executor will need to handle this specially
    {
        let mut fut = future_ref.borrow_mut();
        // We need to store the stream handle somehow - for now we'll use a global registry
        // This is a simplified approach; a production system would use a better method
        fut.metadata = Some(Value::TcpStream(Arc::clone(stream)));
    }

    Ok(Value::Future(future_ref))
}

/// stream.read_exact(num_bytes) - Read exactly num_bytes from the stream (async)
fn tcp_stream_read_exact(stream: &Arc<TcpStreamWrapper>, args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("read_exact requires num_bytes argument".to_string());
    }

    let num_bytes = match &args[0] {
        Value::Int(n) if *n > 0 => *n as usize,
        Value::Int(n) => return Err(format!("read_exact num_bytes must be positive, got {n}")),
        _ => {
            return Err(format!(
                "read_exact num_bytes must be Int, got {}",
                args[0].type_name()
            ))
        }
    };

    let future = FutureState::pending_with_metadata(
        Value::Map(Rc::new(RefCell::new({
            let mut m = HashMap::new();
            m.insert(
                HashableValue::String(Rc::new("num_bytes".into())),
                Value::Int(num_bytes as i64),
            );
            m
        }))),
        "tcp_read_exact".to_string(),
    );
    let future_ref = Rc::new(RefCell::new(future));
    future_ref.borrow_mut().metadata = Some(Value::TcpStream(Arc::clone(stream)));

    Ok(Value::Future(future_ref))
}

/// stream.write(data) - Write data to the stream (async)
/// data can be String or List of bytes
fn tcp_stream_write(stream: &Arc<TcpStreamWrapper>, args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("write requires data argument".to_string());
    }

    let data = match &args[0] {
        Value::String(s) => Value::String(Rc::clone(s)),
        Value::List(l) => Value::List(Rc::clone(l)),
        _ => {
            return Err(format!(
                "write data must be String or List, got {}",
                args[0].type_name()
            ))
        }
    };

    // Store stream in metadata, data in result for the executor to use
    let mut future = FutureState::pending_with_metadata(
        Value::TcpStream(Arc::clone(stream)),
        "tcp_write".to_string(),
    );
    future.result = Some(data); // Store data to write in result field
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// stream.close() - Close the stream
fn tcp_stream_close(_stream: &Arc<TcpStreamWrapper>) -> NativeResult {
    // The stream will be closed when the Arc is dropped
    // For explicit close, we'd need to track closure state
    Ok(Value::Null)
}

/// Methods on TcpListener value type
pub fn tcp_listener_method(
    listener: &Arc<TcpListenerWrapper>,
    method: &str,
    _args: &[Value],
) -> NativeResult {
    match method {
        "accept" => tcp_listener_accept(listener),
        "local_addr" => Ok(Value::string(&listener.local_addr)),
        "close" => Ok(Value::Null), // Will be closed on drop
        _ => Err(format!("TcpListener has no method '{method}'")),
    }
}

/// listener.accept() - Accept a new connection (async)
fn tcp_listener_accept(listener: &Arc<TcpListenerWrapper>) -> NativeResult {
    let future = FutureState::pending_with_metadata(
        Value::TcpListener(Arc::clone(listener)),
        "tcp_accept".to_string(),
    );
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

// ============================================================================
// UDP Module - UDP networking
// ============================================================================

pub fn udp_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "bind" => udp_bind(args),
        _ => Err(format!("Udp has no method '{method}'")),
    }
}

/// Udp.bind(addr, port) - Create a UDP socket bound to the given address (async)
fn udp_bind(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "Udp.bind() expects 2 arguments (addr, port), got {}",
            args.len()
        ));
    }

    let addr = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => {
            return Err(format!(
                "Udp.bind() addr must be String, got {}",
                args[0].type_name()
            ))
        }
    };

    let port = match &args[1] {
        Value::Int(p) if *p >= 0 && *p <= 65535 => *p as u16,
        Value::Int(p) => return Err(format!("Udp.bind() port must be 0-65535, got {p}")),
        _ => {
            return Err(format!(
                "Udp.bind() port must be Int, got {}",
                args[1].type_name()
            ))
        }
    };

    let metadata = Value::string(format!("{addr}:{port}"));
    let future = FutureState::pending_with_metadata(metadata, "udp_bind".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Methods on UdpSocket value type
pub fn udp_socket_method(
    socket: &Arc<UdpSocketWrapper>,
    method: &str,
    args: &[Value],
) -> NativeResult {
    match method {
        "send_to" => udp_socket_send_to(socket, args),
        "recv_from" => udp_socket_recv_from(socket, args),
        "local_addr" => Ok(Value::string(&socket.local_addr)),
        "close" => Ok(Value::Null), // Will be closed on drop
        _ => Err(format!("UdpSocket has no method '{method}'")),
    }
}

/// socket.send_to(data, host, port) - Send data to a specific address (async)
fn udp_socket_send_to(socket: &Arc<UdpSocketWrapper>, args: &[Value]) -> NativeResult {
    if args.len() < 3 {
        return Err(format!(
            "send_to expects 3 arguments (data, host, port), got {}",
            args.len()
        ));
    }

    let data = match &args[0] {
        Value::String(s) => Value::String(Rc::clone(s)),
        Value::List(l) => Value::List(Rc::clone(l)),
        _ => {
            return Err(format!(
                "send_to data must be String or List, got {}",
                args[0].type_name()
            ))
        }
    };

    let host = match &args[1] {
        Value::String(s) => (**s).clone(),
        _ => {
            return Err(format!(
                "send_to host must be String, got {}",
                args[1].type_name()
            ))
        }
    };

    let port = match &args[2] {
        Value::Int(p) if *p > 0 && *p <= 65535 => *p as u16,
        Value::Int(p) => return Err(format!("send_to port must be 1-65535, got {p}")),
        _ => {
            return Err(format!(
                "send_to port must be Int, got {}",
                args[2].type_name()
            ))
        }
    };

    // Store socket in metadata, data/addr map in result for the executor
    let data_map = Value::Map(Rc::new(RefCell::new({
        let mut m = HashMap::new();
        m.insert(HashableValue::String(Rc::new("data".into())), data);
        m.insert(
            HashableValue::String(Rc::new("addr".into())),
            Value::string(format!("{host}:{port}")),
        );
        m
    })));

    let mut future = FutureState::pending_with_metadata(
        Value::UdpSocket(Arc::clone(socket)),
        "udp_send_to".to_string(),
    );
    future.result = Some(data_map); // Store data/addr in result field
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// socket.recv_from(max_bytes?) - Receive data from any sender (async)
/// Returns a map with 'data', 'host', 'port'
fn udp_socket_recv_from(socket: &Arc<UdpSocketWrapper>, args: &[Value]) -> NativeResult {
    let max_bytes = if args.is_empty() {
        65535 // Max UDP datagram size
    } else {
        match &args[0] {
            Value::Int(n) if *n > 0 => *n as usize,
            Value::Int(n) => return Err(format!("recv_from max_bytes must be positive, got {n}")),
            _ => {
                return Err(format!(
                    "recv_from max_bytes must be Int, got {}",
                    args[0].type_name()
                ))
            }
        }
    };

    let metadata = Value::Int(max_bytes as i64);
    let future = FutureState::pending_with_metadata(metadata, "udp_recv_from".to_string());
    let future_ref = Rc::new(RefCell::new(future));
    future_ref.borrow_mut().metadata = Some(Value::UdpSocket(Arc::clone(socket)));

    Ok(Value::Future(future_ref))
}

// ============================================================================
// WebSocket Module - WebSocket client and server support
// ============================================================================

/// WebSocket namespace methods
pub fn ws_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "connect" => ws_connect(args),
        "listen" | "server" => ws_listen(args),
        _ => Err(format!("WebSocket has no method '{method}'")),
    }
}

/// WebSocket.connect(url) - Connect to a WebSocket server (async)
/// Returns a Future<WebSocket>
fn ws_connect(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("WebSocket.connect() expects 1 argument (url)".to_string());
    }

    let url = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => {
            return Err(format!(
                "WebSocket.connect() url must be String, got {}",
                args[0].type_name()
            ))
        }
    };

    // Validate URL scheme
    if !url.starts_with("ws://") && !url.starts_with("wss://") {
        return Err(format!(
            "WebSocket.connect() url must start with ws:// or wss://, got '{url}'"
        ));
    }

    // Store URL as metadata for the executor to use
    let metadata = Value::string(url);
    let future = FutureState::pending_with_metadata(metadata, "ws_connect".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// WebSocket.listen(addr, port) - Create a WebSocket server (async)
/// Returns a Future<WebSocketServer>
fn ws_listen(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "WebSocket.listen() expects 2 arguments (addr, port), got {}",
            args.len()
        ));
    }

    let addr = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => {
            return Err(format!(
                "WebSocket.listen() addr must be String, got {}",
                args[0].type_name()
            ))
        }
    };

    let port = match &args[1] {
        Value::Int(p) if *p >= 0 && *p <= 65535 => *p as u16,
        Value::Int(p) => return Err(format!("WebSocket.listen() port must be 0-65535, got {p}")),
        _ => {
            return Err(format!(
                "WebSocket.listen() port must be Int, got {}",
                args[1].type_name()
            ))
        }
    };

    // Store addr:port as metadata for the executor to use
    let metadata = Value::string(format!("{addr}:{port}"));
    let future = FutureState::pending_with_metadata(metadata, "ws_listen".to_string());
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Methods on WebSocket client value type
pub fn websocket_method(ws: &Arc<WebSocketWrapper>, method: &str, args: &[Value]) -> NativeResult {
    match method {
        "send" => ws_send(ws, args),
        "send_text" => ws_send_text(ws, args),
        "send_binary" => ws_send_binary(ws, args),
        "receive" | "recv" => ws_receive(ws),
        "close" => ws_close(ws),
        "url" => Ok(Value::string(&ws.url)),
        "is_closed" => Ok(Value::Bool(ws.is_closed())),
        _ => Err(format!("WebSocket has no method '{method}'")),
    }
}

/// ws.send(message) - Send a message (text or binary) to the WebSocket server (async)
fn ws_send(ws: &Arc<WebSocketWrapper>, args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("send requires a message argument".to_string());
    }

    if ws.is_closed() {
        return Err("WebSocket is closed".to_string());
    }

    let message = match &args[0] {
        Value::String(s) => Value::String(Rc::clone(s)),
        Value::List(l) => Value::List(Rc::clone(l)), // Binary data as list of bytes
        _ => {
            return Err(format!(
                "send message must be String or List of bytes, got {}",
                args[0].type_name()
            ))
        }
    };

    // Store WebSocket in metadata, message in result
    let mut future =
        FutureState::pending_with_metadata(Value::WebSocket(Arc::clone(ws)), "ws_send".to_string());
    future.result = Some(message);
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// ws.send_text(text) - Send a text message (async)
fn ws_send_text(ws: &Arc<WebSocketWrapper>, args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("send_text requires a text argument".to_string());
    }

    if ws.is_closed() {
        return Err("WebSocket is closed".to_string());
    }

    let text = match &args[0] {
        Value::String(s) => Value::String(Rc::clone(s)),
        _ => {
            return Err(format!(
                "send_text message must be String, got {}",
                args[0].type_name()
            ))
        }
    };

    let mut future = FutureState::pending_with_metadata(
        Value::WebSocket(Arc::clone(ws)),
        "ws_send_text".to_string(),
    );
    future.result = Some(text);
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// ws.send_binary(data) - Send a binary message (async)
fn ws_send_binary(ws: &Arc<WebSocketWrapper>, args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("send_binary requires a data argument".to_string());
    }

    if ws.is_closed() {
        return Err("WebSocket is closed".to_string());
    }

    let data = match &args[0] {
        Value::List(l) => Value::List(Rc::clone(l)),
        _ => {
            return Err(format!(
                "send_binary data must be List of bytes, got {}",
                args[0].type_name()
            ))
        }
    };

    let mut future = FutureState::pending_with_metadata(
        Value::WebSocket(Arc::clone(ws)),
        "ws_send_binary".to_string(),
    );
    future.result = Some(data);
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// ws.receive() - Receive the next message from the WebSocket (async)
/// Returns a map with 'type' ("text" or "binary") and 'data' (String or List)
fn ws_receive(ws: &Arc<WebSocketWrapper>) -> NativeResult {
    if ws.is_closed() {
        return Err("WebSocket is closed".to_string());
    }

    let future = FutureState::pending_with_metadata(
        Value::WebSocket(Arc::clone(ws)),
        "ws_receive".to_string(),
    );
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// ws.close() - Close the WebSocket connection (async)
fn ws_close(ws: &Arc<WebSocketWrapper>) -> NativeResult {
    if ws.is_closed() {
        return Ok(Value::Null); // Already closed
    }

    let future = FutureState::pending_with_metadata(
        Value::WebSocket(Arc::clone(ws)),
        "ws_close".to_string(),
    );
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Methods on WebSocketServer value type
pub fn websocket_server_method(
    server: &Arc<WebSocketServerWrapper>,
    method: &str,
    _args: &[Value],
) -> NativeResult {
    match method {
        "accept" => ws_server_accept(server),
        "local_addr" | "addr" => Ok(Value::string(&server.local_addr)),
        "close" => Ok(Value::Null), // Will be closed on drop
        _ => Err(format!("WebSocketServer has no method '{method}'")),
    }
}

/// server.accept() - Accept a new WebSocket connection (async)
/// Returns a Future<WebSocketServerConn>
fn ws_server_accept(server: &Arc<WebSocketServerWrapper>) -> NativeResult {
    let future = FutureState::pending_with_metadata(
        Value::WebSocketServer(Arc::clone(server)),
        "ws_accept".to_string(),
    );
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// Methods on WebSocketServerConn value type (accepted connection from server)
pub fn websocket_server_conn_method(
    conn: &Arc<WebSocketServerConnWrapper>,
    method: &str,
    args: &[Value],
) -> NativeResult {
    match method {
        "send" => ws_conn_send(conn, args),
        "send_text" => ws_conn_send_text(conn, args),
        "send_binary" => ws_conn_send_binary(conn, args),
        "receive" | "recv" => ws_conn_receive(conn),
        "close" => ws_conn_close(conn),
        "peer_addr" => Ok(Value::string(&conn.peer_addr)),
        "local_addr" => Ok(Value::string(&conn.local_addr)),
        "is_closed" => Ok(Value::Bool(conn.is_closed())),
        _ => Err(format!("WebSocketServerConn has no method '{method}'")),
    }
}

/// conn.send(message) - Send a message to the connected client (async)
fn ws_conn_send(conn: &Arc<WebSocketServerConnWrapper>, args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("send requires a message argument".to_string());
    }

    if conn.is_closed() {
        return Err("WebSocket connection is closed".to_string());
    }

    let message = match &args[0] {
        Value::String(s) => Value::String(Rc::clone(s)),
        Value::List(l) => Value::List(Rc::clone(l)),
        _ => {
            return Err(format!(
                "send message must be String or List of bytes, got {}",
                args[0].type_name()
            ))
        }
    };

    let mut future = FutureState::pending_with_metadata(
        Value::WebSocketServerConn(Arc::clone(conn)),
        "ws_conn_send".to_string(),
    );
    future.result = Some(message);
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// conn.send_text(text) - Send a text message (async)
fn ws_conn_send_text(conn: &Arc<WebSocketServerConnWrapper>, args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("send_text requires a text argument".to_string());
    }

    if conn.is_closed() {
        return Err("WebSocket connection is closed".to_string());
    }

    let text = match &args[0] {
        Value::String(s) => Value::String(Rc::clone(s)),
        _ => {
            return Err(format!(
                "send_text message must be String, got {}",
                args[0].type_name()
            ))
        }
    };

    let mut future = FutureState::pending_with_metadata(
        Value::WebSocketServerConn(Arc::clone(conn)),
        "ws_conn_send_text".to_string(),
    );
    future.result = Some(text);
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// conn.send_binary(data) - Send a binary message (async)
fn ws_conn_send_binary(conn: &Arc<WebSocketServerConnWrapper>, args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("send_binary requires a data argument".to_string());
    }

    if conn.is_closed() {
        return Err("WebSocket connection is closed".to_string());
    }

    let data = match &args[0] {
        Value::List(l) => Value::List(Rc::clone(l)),
        _ => {
            return Err(format!(
                "send_binary data must be List of bytes, got {}",
                args[0].type_name()
            ))
        }
    };

    let mut future = FutureState::pending_with_metadata(
        Value::WebSocketServerConn(Arc::clone(conn)),
        "ws_conn_send_binary".to_string(),
    );
    future.result = Some(data);
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// conn.receive() - Receive the next message from the client (async)
fn ws_conn_receive(conn: &Arc<WebSocketServerConnWrapper>) -> NativeResult {
    if conn.is_closed() {
        return Err("WebSocket connection is closed".to_string());
    }

    let future = FutureState::pending_with_metadata(
        Value::WebSocketServerConn(Arc::clone(conn)),
        "ws_conn_receive".to_string(),
    );
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

/// conn.close() - Close the connection (async)
fn ws_conn_close(conn: &Arc<WebSocketServerConnWrapper>) -> NativeResult {
    if conn.is_closed() {
        return Ok(Value::Null);
    }

    let future = FutureState::pending_with_metadata(
        Value::WebSocketServerConn(Arc::clone(conn)),
        "ws_conn_close".to_string(),
    );
    Ok(Value::Future(Rc::new(RefCell::new(future))))
}

// ============================================================================
// Data Module - DataFrame and Series creation
// ============================================================================

pub fn data_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "frame" | "dataframe" => data_frame(args),
        "series" => data_series(args),
        "from_columns" => data_from_columns(args),
        // Concatenation
        "concat" => data_concat(args),
        // File I/O - readers
        "read_parquet" => data_read_parquet(args),
        "read_csv" => data_read_csv(args),
        "read_json" => data_read_json(args),
        // File I/O - writers
        "write_parquet" => data_write_parquet(args),
        "write_csv" => data_write_csv(args),
        "write_json" => data_write_json(args),
        // SQL operations
        "sql" => data_sql(args),
        "sql_context" => data_sql_context(args),
        // Database query to DataFrame
        "from_query" => data_from_query(args),
        // Parallel configuration
        "set_parallel_threshold" => data_set_parallel_threshold(args),
        "parallel_threshold" => data_parallel_threshold(args),
        _ => Err(format!("Data has no method '{method}'")),
    }
}

/// Set the parallel processing threshold
fn data_set_parallel_threshold(args: &[Value]) -> NativeResult {
    use crate::data::set_parallel_threshold;

    if args.len() != 1 {
        return Err(format!(
            "Data.set_parallel_threshold expects 1 argument, got {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::Int(n) => {
            set_parallel_threshold(*n as usize);
            Ok(Value::Null)
        }
        _ => Err("Data.set_parallel_threshold expects an Int".to_string()),
    }
}

/// Get the current parallel processing threshold
fn data_parallel_threshold(_args: &[Value]) -> NativeResult {
    use crate::data::parallel_threshold;

    Ok(Value::Int(parallel_threshold() as i64))
}

/// Create a DataFrame from a list of maps (each map is a row)
fn data_frame(args: &[Value]) -> NativeResult {
    use std::sync::Arc;

    if args.is_empty() {
        return Err("Data.frame requires at least one row".to_string());
    }

    // Handle list of rows (maps)
    let rows = match &args[0] {
        Value::List(list) => list.borrow().clone(),
        _ => return Err("Data.frame expects a List of Map rows".to_string()),
    };

    if rows.is_empty() {
        return Err("Data.frame requires at least one row".to_string());
    }

    // Get column names from the first row
    let first_row = match &rows[0] {
        Value::Map(map) => map.borrow().clone(),
        _ => return Err("Each row must be a Map".to_string()),
    };

    let column_names: Vec<String> = first_row
        .keys()
        .filter_map(|k| match k {
            HashableValue::String(s) => Some((**s).clone()),
            _ => None,
        })
        .collect();

    if column_names.is_empty() {
        return Err("Rows must have at least one string-keyed column".to_string());
    }

    // Build columns from all rows
    let mut column_values: Vec<Vec<Value>> = vec![Vec::new(); column_names.len()];

    for row_val in &rows {
        let row = match row_val {
            Value::Map(map) => map.borrow().clone(),
            _ => return Err("Each row must be a Map".to_string()),
        };

        for (col_idx, col_name) in column_names.iter().enumerate() {
            let key = HashableValue::String(Rc::new(col_name.clone()));
            let value = row.get(&key).cloned().unwrap_or(Value::Null);
            column_values[col_idx].push(value);
        }
    }

    // Convert to Series
    let series_list: Result<Vec<Series>, _> = column_names
        .iter()
        .zip(column_values.iter())
        .map(|(name, values)| Series::from_values(name, values))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());

    let series_list = series_list?;

    // Create DataFrame
    let df = DataFrame::from_series(series_list).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(Arc::new(df)))
}

/// Create a Series from a name and list of values
fn data_series(args: &[Value]) -> NativeResult {
    use std::sync::Arc;

    if args.len() != 2 {
        return Err("Data.series expects 2 arguments: name and values".to_string());
    }

    let name = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("First argument must be a String (column name)".to_string()),
    };

    let values = match &args[1] {
        Value::List(list) => list.borrow().clone(),
        _ => return Err("Second argument must be a List of values".to_string()),
    };

    let series = Series::from_values(&name, &values).map_err(|e| e.to_string())?;
    Ok(Value::Series(Arc::new(series)))
}

/// Create a DataFrame from named columns: Data.from_columns("a", [1,2,3], "b", [4,5,6])
fn data_from_columns(args: &[Value]) -> NativeResult {
    use std::sync::Arc;

    if args.len() < 2 || args.len() % 2 != 0 {
        return Err("Data.from_columns expects pairs of (name, values)".to_string());
    }

    let mut series_list = Vec::new();

    for pair in args.chunks(2) {
        let name = match &pair[0] {
            Value::String(s) => (**s).clone(),
            _ => return Err("Column name must be a String".to_string()),
        };

        let values = match &pair[1] {
            Value::List(list) => list.borrow().clone(),
            _ => return Err("Column values must be a List".to_string()),
        };

        let series = Series::from_values(&name, &values).map_err(|e| e.to_string())?;
        series_list.push(series);
    }

    let df = DataFrame::from_series(series_list).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(Arc::new(df)))
}

/// Concatenate multiple DataFrames vertically: Data.concat(df1, df2, ...)
fn data_concat(args: &[Value]) -> NativeResult {
    use std::sync::Arc;

    if args.is_empty() {
        return Err("Data.concat expects at least one DataFrame".to_string());
    }

    // Collect all DataFrames from arguments
    let dataframes: Result<Vec<&DataFrame>, String> = args
        .iter()
        .enumerate()
        .map(|(i, v)| match v {
            Value::DataFrame(df) => Ok(df.as_ref()),
            _ => Err(format!(
                "Data.concat argument {} must be a DataFrame, got {}",
                i,
                v.type_name()
            )),
        })
        .collect();

    let dataframes = dataframes?;

    let result = DataFrame::concat(&dataframes).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(Arc::new(result)))
}

// ============================================================================
// Data Module - File I/O
// ============================================================================

/// Data.read_parquet(path) - Read a Parquet file into a DataFrame
fn data_read_parquet(args: &[Value]) -> NativeResult {
    use std::sync::Arc;

    if args.len() != 1 {
        return Err("Data.read_parquet expects 1 argument: path".to_string());
    }

    let path = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Data.read_parquet expects a String path".to_string()),
    };

    let df = read_parquet(&path).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(Arc::new(df)))
}

/// Data.read_csv(path) or Data.read_csv(path, has_header, delimiter) - Read a CSV file into a DataFrame
fn data_read_csv(args: &[Value]) -> NativeResult {
    use std::sync::Arc;

    if args.is_empty() || args.len() > 3 {
        return Err(
            "Data.read_csv expects 1-3 arguments: path, [has_header], [delimiter]".to_string(),
        );
    }

    let path = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Data.read_csv expects a String path".to_string()),
    };

    let has_header = if args.len() >= 2 {
        match &args[1] {
            Value::Bool(b) => *b,
            _ => return Err("has_header must be a Bool".to_string()),
        }
    } else {
        true
    };

    let delimiter = if args.len() >= 3 {
        match &args[2] {
            Value::String(s) => {
                if s.len() != 1 {
                    return Err("delimiter must be a single character".to_string());
                }
                s.bytes().next().unwrap_or(b',')
            }
            _ => return Err("delimiter must be a String".to_string()),
        }
    } else {
        b','
    };

    let df = read_csv_with_options(&path, has_header, delimiter).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(Arc::new(df)))
}

/// Data.read_json(path) - Read a JSON file (newline-delimited) into a DataFrame
fn data_read_json(args: &[Value]) -> NativeResult {
    use std::sync::Arc;

    if args.len() != 1 {
        return Err("Data.read_json expects 1 argument: path".to_string());
    }

    let path = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Data.read_json expects a String path".to_string()),
    };

    let df = read_json(&path).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(Arc::new(df)))
}

/// Data.write_parquet(df, path) - Write a DataFrame to a Parquet file
fn data_write_parquet(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("Data.write_parquet expects 2 arguments: df, path".to_string());
    }

    let df = match &args[0] {
        Value::DataFrame(df) => df.clone(),
        _ => return Err("First argument must be a DataFrame".to_string()),
    };

    let path = match &args[1] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Second argument must be a String path".to_string()),
    };

    write_parquet(&df, &path).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// Data.write_csv(df, path) - Write a DataFrame to a CSV file
fn data_write_csv(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("Data.write_csv expects 2 arguments: df, path".to_string());
    }

    let df = match &args[0] {
        Value::DataFrame(df) => df.clone(),
        _ => return Err("First argument must be a DataFrame".to_string()),
    };

    let path = match &args[1] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Second argument must be a String path".to_string()),
    };

    write_csv(&df, &path).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// Data.write_json(df, path) - Write a DataFrame to a JSON file
fn data_write_json(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("Data.write_json expects 2 arguments: df, path".to_string());
    }

    let df = match &args[0] {
        Value::DataFrame(df) => df.clone(),
        _ => return Err("First argument must be a DataFrame".to_string()),
    };

    let path = match &args[1] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Second argument must be a String path".to_string()),
    };

    write_json(&df, &path).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

// ============================================================================
// Data Module - SQL Operations
// ============================================================================

/// Data.sql(df, query) - Execute SQL query against a single DataFrame
fn data_sql(args: &[Value]) -> NativeResult {
    use std::sync::Arc;

    if args.len() != 2 {
        return Err("Data.sql expects 2 arguments: df, query".to_string());
    }

    let df = match &args[0] {
        Value::DataFrame(df) => df.clone(),
        _ => return Err("First argument must be a DataFrame".to_string()),
    };

    let query = match &args[1] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Second argument must be a SQL query String".to_string()),
    };

    let result = sql_query(&df, &query).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(Arc::new(result)))
}

/// Data.sql_context() - Create a new SQL context for multi-table queries
fn data_sql_context(_args: &[Value]) -> NativeResult {
    let ctx = SqlContext::new().map_err(|e| e.to_string())?;
    Ok(Value::SqlContext(std::sync::Arc::new(
        std::sync::Mutex::new(ctx),
    )))
}

/// Data.from_query(db, sql, params?) - Execute SQL query against database and return DataFrame
///
/// This is a convenience function that combines database query with DataFrame creation.
/// ```stratum
/// let db = Db.sqlite(":memory:")
/// db.execute("CREATE TABLE users (id INT, name TEXT)")
/// db.execute("INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob')")
/// let df = Data.from_query(db, "SELECT * FROM users")
/// ```
fn data_from_query(args: &[Value]) -> NativeResult {
    use std::sync::Arc;

    if args.is_empty() || args.len() > 3 {
        return Err("Data.from_query expects 2-3 arguments: db, sql, [params]".to_string());
    }

    let conn = match &args[0] {
        Value::DbConnection(conn) => conn.clone(),
        _ => return Err("First argument must be a DbConnection".to_string()),
    };

    let sql = match &args[1] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Second argument must be a SQL query String".to_string()),
    };

    let params = if args.len() > 2 {
        extract_params(&args[2])?
    } else {
        Vec::new()
    };

    // Execute the query and get results as List of Maps
    let query_result = match &conn.kind {
        DbConnectionKind::Sqlite(c) => sqlite_query(c, &sql, &params),
        DbConnectionKind::Postgres(c) => postgres_query(c, &sql, &params),
        DbConnectionKind::MySql(c) => mysql_query(c, &sql, &params),
        DbConnectionKind::DuckDb(c) => duckdb_query(c, &sql, &params),
    }?;

    // Convert the List of Maps to a DataFrame
    let rows = match query_result {
        Value::List(list) => list.borrow().clone(),
        _ => return Err("Query did not return a list".to_string()),
    };

    // Handle empty results - return empty DataFrame
    if rows.is_empty() {
        let df = DataFrame::from_series(vec![]).map_err(|e| e.to_string())?;
        return Ok(Value::DataFrame(Arc::new(df)));
    }

    // Get column names from the first row
    let first_row = match &rows[0] {
        Value::Map(map) => map.borrow().clone(),
        _ => return Err("Query results must be a list of maps".to_string()),
    };

    let column_names: Vec<String> = first_row
        .keys()
        .filter_map(|k| match k {
            HashableValue::String(s) => Some((**s).clone()),
            _ => None,
        })
        .collect();

    if column_names.is_empty() {
        let df = DataFrame::from_series(vec![]).map_err(|e| e.to_string())?;
        return Ok(Value::DataFrame(Arc::new(df)));
    }

    // Build columns from all rows
    let mut column_values: Vec<Vec<Value>> = vec![Vec::new(); column_names.len()];

    for row_val in &rows {
        let row = match row_val {
            Value::Map(map) => map.borrow().clone(),
            _ => return Err("Each row must be a Map".to_string()),
        };

        for (col_idx, col_name) in column_names.iter().enumerate() {
            let key = HashableValue::String(Rc::new(col_name.clone()));
            let value = row.get(&key).cloned().unwrap_or(Value::Null);
            column_values[col_idx].push(value);
        }
    }

    // Convert to Series
    let series_list: Result<Vec<Series>, _> = column_names
        .iter()
        .zip(column_values.iter())
        .map(|(name, values)| Series::from_values(name, values))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());

    let series_list = series_list?;

    // Create DataFrame
    let df = DataFrame::from_series(series_list).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(Arc::new(df)))
}

// ============================================================================
// SqlContext Methods
// ============================================================================

/// Method dispatch for SqlContext values
pub fn sql_context_method(
    ctx: &std::sync::Arc<std::sync::Mutex<SqlContext>>,
    method: &str,
    args: &[Value],
) -> NativeResult {
    match method {
        "register" => sql_context_register(ctx, args),
        "query" | "sql" => sql_context_query(ctx, args),
        "tables" => sql_context_tables(ctx),
        _ => Err(format!("SqlContext has no method '{method}'")),
    }
}

/// ctx.register(name, df) - Register a DataFrame as a table
fn sql_context_register(
    ctx: &std::sync::Arc<std::sync::Mutex<SqlContext>>,
    args: &[Value],
) -> NativeResult {
    if args.len() != 2 {
        return Err("register expects 2 arguments: name, df".to_string());
    }

    let name = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("First argument must be a String (table name)".to_string()),
    };

    let df = match &args[1] {
        Value::DataFrame(df) => df.clone(),
        _ => return Err("Second argument must be a DataFrame".to_string()),
    };

    let guard = ctx.lock().map_err(|e| format!("Lock error: {e}"))?;
    guard.register(&name, &df).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// ctx.query(sql) or ctx.sql(sql) - Execute a SQL query
fn sql_context_query(
    ctx: &std::sync::Arc<std::sync::Mutex<SqlContext>>,
    args: &[Value],
) -> NativeResult {
    if args.len() != 1 {
        return Err("query expects 1 argument: sql".to_string());
    }

    let sql = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Argument must be a SQL query String".to_string()),
    };

    let guard = ctx.lock().map_err(|e| format!("Lock error: {e}"))?;
    let result = guard.query(&sql).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(std::sync::Arc::new(result)))
}

/// ctx.tables() - Get list of registered table names
fn sql_context_tables(ctx: &std::sync::Arc<std::sync::Mutex<SqlContext>>) -> NativeResult {
    let guard = ctx.lock().map_err(|e| format!("Lock error: {e}"))?;
    let tables = guard.tables();
    let list: Vec<Value> = tables
        .into_iter()
        .map(|s| Value::String(std::rc::Rc::new(s)))
        .collect();
    Ok(Value::List(std::rc::Rc::new(std::cell::RefCell::new(list))))
}

// ============================================================================
// Agg Module - Aggregation specification builders
// ============================================================================

/// Aggregation builder methods for creating AggSpec values
pub fn agg_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "sum" => agg_sum(args),
        "mean" | "avg" => agg_mean(args),
        "min" => agg_min(args),
        "max" => agg_max(args),
        "count" => agg_count(args),
        "first" => agg_first(args),
        "last" => agg_last(args),
        "std" | "stddev" => agg_std(args),
        "var" | "variance" => agg_var(args),
        "median" => agg_median(args),
        "mode" => agg_mode(args),
        "count_distinct" | "nunique" => agg_count_distinct(args),
        _ => Err(format!("Agg has no method '{method}'")),
    }
}

/// Agg.sum("column", "output_name") or Agg.sum("column") - creates a sum aggregation spec
fn agg_sum(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "sum")?;
    let spec = AggSpec::new(AggOp::Sum, Some(column.clone()), output.unwrap_or(column));
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.mean("column", "output_name") - creates a mean aggregation spec
fn agg_mean(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "mean")?;
    let spec = AggSpec::new(AggOp::Mean, Some(column.clone()), output.unwrap_or(column));
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.min("column", "output_name") - creates a min aggregation spec
fn agg_min(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "min")?;
    let spec = AggSpec::new(AggOp::Min, Some(column.clone()), output.unwrap_or(column));
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.max("column", "output_name") - creates a max aggregation spec
fn agg_max(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "max")?;
    let spec = AggSpec::new(AggOp::Max, Some(column.clone()), output.unwrap_or(column));
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.count("output_name") - creates a count aggregation spec
fn agg_count(args: &[Value]) -> NativeResult {
    let output = if args.is_empty() {
        "count".to_string()
    } else if args.len() == 1 {
        match &args[0] {
            Value::String(s) => (**s).clone(),
            _ => return Err("Agg.count expects a String output name".to_string()),
        }
    } else {
        return Err("Agg.count expects 0 or 1 arguments".to_string());
    };
    let spec = AggSpec::new(AggOp::Count, None, output);
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.first("column", "output_name") - creates a first aggregation spec
fn agg_first(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "first")?;
    let spec = AggSpec::new(AggOp::First, Some(column.clone()), output.unwrap_or(column));
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.last("column", "output_name") - creates a last aggregation spec
fn agg_last(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "last")?;
    let spec = AggSpec::new(AggOp::Last, Some(column.clone()), output.unwrap_or(column));
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.std("column", "output_name") - creates a standard deviation aggregation spec
fn agg_std(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "std")?;
    let spec = AggSpec::new(AggOp::Std, Some(column.clone()), output.unwrap_or(column));
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.var("column", "output_name") - creates a variance aggregation spec
fn agg_var(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "var")?;
    let spec = AggSpec::new(AggOp::Var, Some(column.clone()), output.unwrap_or(column));
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.median("column", "output_name") - creates a median aggregation spec
fn agg_median(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "median")?;
    let spec = AggSpec::new(
        AggOp::Median,
        Some(column.clone()),
        output.unwrap_or(column),
    );
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.mode("column", "output_name") - creates a mode aggregation spec
fn agg_mode(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "mode")?;
    let spec = AggSpec::new(AggOp::Mode, Some(column.clone()), output.unwrap_or(column));
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Agg.count_distinct("column", "output_name") - creates a count distinct aggregation spec
fn agg_count_distinct(args: &[Value]) -> NativeResult {
    let (column, output) = parse_agg_args(args, "count_distinct")?;
    let spec = AggSpec::new(
        AggOp::CountDistinct,
        Some(column.clone()),
        output.unwrap_or(column),
    );
    Ok(Value::AggSpec(std::sync::Arc::new(spec)))
}

/// Parse aggregation arguments: (column) or (column, output_name)
fn parse_agg_args(args: &[Value], method: &str) -> Result<(String, Option<String>), String> {
    if args.is_empty() || args.len() > 2 {
        return Err(format!("Agg.{method} expects 1 or 2 arguments"));
    }

    let column = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => {
            return Err(format!(
                "Agg.{method} first argument must be a column name (String)"
            ))
        }
    };

    let output = if args.len() == 2 {
        match &args[1] {
            Value::String(s) => Some((**s).clone()),
            _ => {
                return Err(format!(
                    "Agg.{method} second argument must be an output name (String)"
                ))
            }
        }
    } else {
        None
    };

    Ok((column, output))
}

// ============================================================================
// Join Module - Builder pattern for DataFrame joins
// ============================================================================

/// Dispatch Join.method calls
pub fn join_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "on" => join_on(args),
        "cols" => join_cols(args),
        "inner" => join_inner(args),
        "inner_cols" => join_inner_cols(args),
        "left" => join_left(args),
        "left_cols" => join_left_cols(args),
        "right" => join_right(args),
        "right_cols" => join_right_cols(args),
        "outer" => join_outer(args),
        "outer_cols" => join_outer_cols(args),
        _ => Err(format!("Join has no method '{method}'")),
    }
}

/// Join.on("column") - creates an inner join spec on the same column name
fn join_on(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err("Join.on expects 1 argument (column name)".to_string());
    }
    let column = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Join.on expects a String column name".to_string()),
    };
    let spec = JoinSpec::on(&column);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Join.cols("left_col", "right_col") - creates an inner join spec on different column names
fn join_cols(args: &[Value]) -> NativeResult {
    let (left, right) = parse_join_cols_args(args, "cols")?;
    let spec = JoinSpec::cols(&left, &right);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Join.inner("column") - explicit inner join on same column name
fn join_inner(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err("Join.inner expects 1 argument (column name)".to_string());
    }
    let column = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Join.inner expects a String column name".to_string()),
    };
    let spec = JoinSpec::inner(&column);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Join.inner_cols("left", "right") - inner join on different columns
fn join_inner_cols(args: &[Value]) -> NativeResult {
    let (left, right) = parse_join_cols_args(args, "inner_cols")?;
    let spec = JoinSpec::inner_cols(&left, &right);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Join.left("column") - left join on same column name
fn join_left(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err("Join.left expects 1 argument (column name)".to_string());
    }
    let column = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Join.left expects a String column name".to_string()),
    };
    let spec = JoinSpec::left(&column);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Join.left_cols("left", "right") - left join on different columns
fn join_left_cols(args: &[Value]) -> NativeResult {
    let (left, right) = parse_join_cols_args(args, "left_cols")?;
    let spec = JoinSpec::left_cols(&left, &right);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Join.right("column") - right join on same column name
fn join_right(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err("Join.right expects 1 argument (column name)".to_string());
    }
    let column = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Join.right expects a String column name".to_string()),
    };
    let spec = JoinSpec::right(&column);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Join.right_cols("left", "right") - right join on different columns
fn join_right_cols(args: &[Value]) -> NativeResult {
    let (left, right) = parse_join_cols_args(args, "right_cols")?;
    let spec = JoinSpec::right_cols(&left, &right);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Join.outer("column") - outer join on same column name
fn join_outer(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err("Join.outer expects 1 argument (column name)".to_string());
    }
    let column = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => return Err("Join.outer expects a String column name".to_string()),
    };
    let spec = JoinSpec::outer(&column);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Join.outer_cols("left", "right") - outer join on different columns
fn join_outer_cols(args: &[Value]) -> NativeResult {
    let (left, right) = parse_join_cols_args(args, "outer_cols")?;
    let spec = JoinSpec::outer_cols(&left, &right);
    Ok(Value::JoinSpec(std::sync::Arc::new(spec)))
}

/// Parse (left_col, right_col) arguments for join methods
fn parse_join_cols_args(args: &[Value], method: &str) -> Result<(String, String), String> {
    if args.len() != 2 {
        return Err(format!(
            "Join.{method} expects 2 arguments (left_column, right_column)"
        ));
    }
    let left = match &args[0] {
        Value::String(s) => (**s).clone(),
        _ => {
            return Err(format!(
                "Join.{method} first argument must be a column name (String)"
            ))
        }
    };
    let right = match &args[1] {
        Value::String(s) => (**s).clone(),
        _ => {
            return Err(format!(
                "Join.{method} second argument must be a column name (String)"
            ))
        }
    };
    Ok((left, right))
}

// ============================================================================
// Cube Module (OLAP Cube for multi-dimensional analysis)
// ============================================================================

/// Dispatch a method call on the Cube namespace
pub fn cube_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "from" => cube_from(args),
        _ => Err(format!("Cube has no method '{method}'")),
    }
}

/// Cube.from(df) or Cube.from("name", df) - Create a CubeBuilder from a DataFrame
fn cube_from(args: &[Value]) -> NativeResult {
    use std::sync::{Arc, Mutex};

    match args.len() {
        1 => {
            // Cube.from(df)
            let df = match &args[0] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "Cube.from expects a DataFrame, got {}",
                        other.type_name()
                    ))
                }
            };
            let builder = CubeBuilder::from_dataframe(df).map_err(|e| e.to_string())?;
            Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(builder)))))
        }
        2 => {
            // Cube.from("name", df)
            let name = match &args[0] {
                Value::String(s) => (**s).clone(),
                other => {
                    return Err(format!(
                        "Cube.from first argument must be a name (String), got {}",
                        other.type_name()
                    ))
                }
            };
            let df = match &args[1] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "Cube.from second argument must be a DataFrame, got {}",
                        other.type_name()
                    ))
                }
            };
            let builder =
                CubeBuilder::from_dataframe_with_name(&name, df).map_err(|e| e.to_string())?;
            Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(builder)))))
        }
        n => Err(format!(
            "Cube.from expects 1 or 2 arguments (DataFrame or name + DataFrame), got {n}"
        )),
    }
}

// ============================================================================
// Set Module
// ============================================================================

/// Set namespace for creating and working with sets
pub fn set_native_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "new" => set_new(args),
        "from" | "from_list" => set_from_list(args),
        _ => Err(format!("Set has no method '{method}'")),
    }
}

/// Set.new() -> Set
/// Create an empty set
fn set_new(args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!("Set.new() expects 0 arguments, got {}", args.len()));
    }
    Ok(Value::empty_set())
}

/// Set.from(list) -> Set
/// Create a set from a list of values
fn set_from_list(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Set.from() expects 1 argument (list), got {}",
            args.len()
        ));
    }

    match &args[0] {
        Value::List(list) => {
            let mut set = std::collections::HashSet::new();
            for value in list.borrow().iter() {
                let hashable = HashableValue::try_from(value.clone()).map_err(|_| {
                    format!(
                        "Set can only contain hashable values (null, bool, int, string), got {}",
                        value.type_name()
                    )
                })?;
                set.insert(hashable);
            }
            Ok(Value::set(set))
        }
        _ => Err(format!(
            "Set.from() expects a List, got {}",
            args[0].type_name()
        )),
    }
}

// ============================================================================
// Test Module - Testing framework for Stratum
// ============================================================================

/// Test namespace for the Stratum testing framework
pub fn test_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "expect" => test_expect(args),
        "fail" => test_fail(args),
        "skip" => test_skip(args),
        "pending" => test_pending(args),
        "mock" => test_mock(args),
        "spy" => test_spy(args),
        _ => Err(format!("Test has no method '{method}'")),
    }
}

/// Test.expect(value) -> Expectation
/// Creates an expectation that can be used with matchers like to_be, to_equal, etc.
fn test_expect(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Test.expect() expects 1 argument, got {}",
            args.len()
        ));
    }
    Ok(Value::expectation(args[0].clone()))
}

/// Test.fail(message?) -> throws
/// Immediately fails the current test with an optional message
fn test_fail(args: &[Value]) -> NativeResult {
    let message = if args.is_empty() {
        "Test failed".to_string()
    } else {
        match &args[0] {
            Value::String(s) => (**s).clone(),
            v => format!("{}", v),
        }
    };
    Err(message)
}

/// Test.skip(message?) -> null
/// Marks the current test as skipped (does not fail)
fn test_skip(args: &[Value]) -> NativeResult {
    let _message = if args.is_empty() {
        "Test skipped".to_string()
    } else {
        match &args[0] {
            Value::String(s) => (**s).clone(),
            v => format!("{}", v),
        }
    };
    // For now, skip just returns null - could add special handling later
    Ok(Value::Null)
}

/// Test.pending(message?) -> null
/// Marks the current test as pending (not yet implemented)
fn test_pending(args: &[Value]) -> NativeResult {
    let _message = if args.is_empty() {
        "Test pending".to_string()
    } else {
        match &args[0] {
            Value::String(s) => (**s).clone(),
            v => format!("{}", v),
        }
    };
    // For now, pending just returns null - could add special handling later
    Ok(Value::Null)
}

/// Test.mock(return_value?) -> Mock
/// Creates a mock function that records calls and returns a configured value.
/// The mock is represented as a Map with the following structure:
/// { "__is_mock": true, "return_value": value, "calls": [], "call_count": 0 }
fn test_mock(args: &[Value]) -> NativeResult {
    let return_value = if args.is_empty() {
        Value::Null
    } else {
        args[0].clone()
    };

    let mut mock = HashMap::new();
    // Marker to identify this as a mock
    mock.insert(
        HashableValue::String(Rc::new("__is_mock".to_string())),
        Value::Bool(true),
    );
    // The value to return when called
    mock.insert(
        HashableValue::String(Rc::new("return_value".to_string())),
        return_value,
    );
    // Record of all calls made (list of argument lists)
    mock.insert(
        HashableValue::String(Rc::new("calls".to_string())),
        Value::empty_list(),
    );
    // Count of calls made
    mock.insert(
        HashableValue::String(Rc::new("call_count".to_string())),
        Value::Int(0),
    );

    Ok(Value::Map(Rc::new(RefCell::new(mock))))
}

/// Test.spy(fn?) -> Spy
/// Creates a spy that wraps a function and tracks calls.
/// If no function is provided, creates a spy that returns null.
fn test_spy(args: &[Value]) -> NativeResult {
    // For now, spy is similar to mock but can optionally wrap a real function
    let wrapped = if args.is_empty() {
        Value::Null
    } else {
        args[0].clone()
    };

    let mut spy = HashMap::new();
    // Marker to identify this as a spy
    spy.insert(
        HashableValue::String(Rc::new("__is_spy".to_string())),
        Value::Bool(true),
    );
    // The wrapped function (if any)
    spy.insert(
        HashableValue::String(Rc::new("wrapped".to_string())),
        wrapped,
    );
    // Record of all calls made
    spy.insert(
        HashableValue::String(Rc::new("calls".to_string())),
        Value::empty_list(),
    );
    // Count of calls made
    spy.insert(
        HashableValue::String(Rc::new("call_count".to_string())),
        Value::Int(0),
    );

    Ok(Value::Map(Rc::new(RefCell::new(spy))))
}

// ============================================================================
// XML Module
// ============================================================================

/// Xml namespace methods (static methods)
pub fn xml_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "parse" => xml_parse(args),
        "stringify" | "serialize" => xml_stringify(args),
        _ => Err(format!("Xml has no method '{method}'")),
    }
}

/// Xml.parse(content: String) -> XmlDocument
/// Parse XML content into a document object
fn xml_parse(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Xml.parse() expects 1 argument, got {}",
            args.len()
        ));
    }
    let content = get_string_arg(&args[0], "content")?;

    // Parse the document to validate it and get root name
    let package = xml_parser::parse(&content).map_err(|e| format!("XML parse error: {}", e))?;
    let doc = package.as_document();

    let root_name = doc
        .root()
        .children()
        .iter()
        .find_map(|child| child.element().map(|e| e.name().local_part().to_string()))
        .unwrap_or_else(|| "document".to_string());

    let wrapper = XmlDocumentWrapper::new(content, root_name);
    Ok(Value::XmlDocument(Arc::new(wrapper)))
}

/// Xml.stringify(doc: XmlDocument) -> String
/// Convert an XmlDocument back to a string
fn xml_stringify(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Xml.stringify() expects 1 argument, got {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::XmlDocument(doc) => Ok(Value::string(&doc.content)),
        other => Err(format!(
            "Xml.stringify() expects XmlDocument, got {}",
            other.type_name()
        )),
    }
}

/// Methods on XmlDocument instances
pub fn xml_document_method(
    doc: &Arc<XmlDocumentWrapper>,
    method: &str,
    args: &[Value],
) -> NativeResult {
    match method {
        "query" | "xpath" => xml_doc_query(doc, args),
        "text" => xml_doc_text(doc, args),
        "root" => xml_doc_root(doc),
        "content" => Ok(Value::string(&doc.content)),
        _ => Err(format!("XmlDocument has no method '{method}'")),
    }
}

/// doc.query(xpath: String) -> Value
/// Execute an XPath query and return results
fn xml_doc_query(doc: &Arc<XmlDocumentWrapper>, args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("query() expects 1 argument, got {}", args.len()));
    }
    let xpath_expr = get_string_arg(&args[0], "xpath")?;

    let package = xml_parser::parse(&doc.content).map_err(|e| format!("XML parse error: {}", e))?;
    let document = package.as_document();

    // Create XPath factory and context
    let factory = Factory::new();
    let xpath = factory
        .build(&xpath_expr)
        .map_err(|e| format!("XPath parse error: {}", e))?
        .ok_or("Empty XPath expression")?;

    let context = Context::new();
    let result = xpath
        .evaluate(&context, document.root())
        .map_err(|e| format!("XPath evaluation error: {}", e))?;

    xpath_value_to_stratum(result)
}

/// Convert XPath result to Stratum value
fn xpath_value_to_stratum(value: XPathValue) -> NativeResult {
    match value {
        XPathValue::Boolean(b) => Ok(Value::Bool(b)),
        XPathValue::Number(n) => {
            if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                Ok(Value::Int(n as i64))
            } else {
                Ok(Value::Float(n))
            }
        }
        XPathValue::String(s) => Ok(Value::string(s)),
        XPathValue::Nodeset(nodes) => {
            let items: Vec<Value> = nodes
                .iter()
                .map(|node| {
                    // Get text content from node
                    let text = node.string_value();
                    Value::string(text)
                })
                .collect();
            Ok(Value::list(items))
        }
    }
}

/// doc.text() -> String
/// Get all text content from the document
fn xml_doc_text(doc: &Arc<XmlDocumentWrapper>, args: &[Value]) -> NativeResult {
    if !args.is_empty() {
        return Err(format!("text() expects 0 arguments, got {}", args.len()));
    }

    let package = xml_parser::parse(&doc.content).map_err(|e| format!("XML parse error: {}", e))?;
    let document = package.as_document();

    // Use XPath to get all text
    let result =
        evaluate_xpath(&document, "//text()").map_err(|e| format!("XPath error: {}", e))?;

    match result {
        XPathValue::Nodeset(nodes) => {
            let text: String = nodes
                .iter()
                .map(|n| n.string_value())
                .collect::<Vec<_>>()
                .join("");
            Ok(Value::string(text.trim()))
        }
        XPathValue::String(s) => Ok(Value::string(s)),
        _ => Ok(Value::string("")),
    }
}

/// doc.root() -> String
/// Get the root element name
fn xml_doc_root(doc: &Arc<XmlDocumentWrapper>) -> NativeResult {
    Ok(Value::string(&doc.root_name))
}

// ============================================================================
// Image Module
// ============================================================================

/// Image namespace methods (static methods)
pub fn image_namespace_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "open" | "load" => image_open(args),
        "new" | "create" => image_new(args),
        _ => Err(format!("Image has no method '{method}'")),
    }
}

/// Image.open(path: String) -> Image
/// Load an image from a file
fn image_open(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Image.open() expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;

    let img = image::open(&path).map_err(|e| format!("failed to open image '{}': {}", path, e))?;

    // Detect format from extension
    let format = std::path::Path::new(&path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase());

    let wrapper = ImageWrapper::new(img, Some(path), format);
    Ok(Value::Image(Arc::new(wrapper)))
}

/// Image.new(width: Int, height: Int, color?: String) -> Image
/// Create a new blank image
fn image_new(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(format!(
            "Image.new() expects at least 2 arguments (width, height), got {}",
            args.len()
        ));
    }

    let width = get_int_arg(&args[0], "width")? as u32;
    let height = get_int_arg(&args[1], "height")? as u32;

    // Parse optional color (default white)
    let color = if args.len() > 2 {
        parse_color(&args[2])?
    } else {
        image::Rgba([255, 255, 255, 255])
    };

    let img = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(width, height, color));
    let wrapper = ImageWrapper::new(img, None, Some("png".to_string()));
    Ok(Value::Image(Arc::new(wrapper)))
}

/// Parse color from value (string like "#RRGGBB" or "white", or list [r,g,b] or [r,g,b,a])
fn parse_color(value: &Value) -> Result<image::Rgba<u8>, String> {
    match value {
        Value::String(s) => {
            let s = s.trim();
            if s.starts_with('#') {
                // Parse hex color
                let hex = s.trim_start_matches('#');
                match hex.len() {
                    6 => {
                        let r =
                            u8::from_str_radix(&hex[0..2], 16).map_err(|_| "invalid hex color")?;
                        let g =
                            u8::from_str_radix(&hex[2..4], 16).map_err(|_| "invalid hex color")?;
                        let b =
                            u8::from_str_radix(&hex[4..6], 16).map_err(|_| "invalid hex color")?;
                        Ok(image::Rgba([r, g, b, 255]))
                    }
                    8 => {
                        let r =
                            u8::from_str_radix(&hex[0..2], 16).map_err(|_| "invalid hex color")?;
                        let g =
                            u8::from_str_radix(&hex[2..4], 16).map_err(|_| "invalid hex color")?;
                        let b =
                            u8::from_str_radix(&hex[4..6], 16).map_err(|_| "invalid hex color")?;
                        let a =
                            u8::from_str_radix(&hex[6..8], 16).map_err(|_| "invalid hex color")?;
                        Ok(image::Rgba([r, g, b, a]))
                    }
                    _ => Err("hex color must be #RRGGBB or #RRGGBBAA".to_string()),
                }
            } else {
                // Named colors
                match s.to_lowercase().as_str() {
                    "white" => Ok(image::Rgba([255, 255, 255, 255])),
                    "black" => Ok(image::Rgba([0, 0, 0, 255])),
                    "red" => Ok(image::Rgba([255, 0, 0, 255])),
                    "green" => Ok(image::Rgba([0, 255, 0, 255])),
                    "blue" => Ok(image::Rgba([0, 0, 255, 255])),
                    "yellow" => Ok(image::Rgba([255, 255, 0, 255])),
                    "cyan" => Ok(image::Rgba([0, 255, 255, 255])),
                    "magenta" => Ok(image::Rgba([255, 0, 255, 255])),
                    "transparent" => Ok(image::Rgba([0, 0, 0, 0])),
                    _ => Err(format!("unknown color name: {}", s)),
                }
            }
        }
        Value::List(list) => {
            let list = list.borrow();
            if list.len() < 3 || list.len() > 4 {
                return Err(
                    "color list must have 3 or 4 elements [r, g, b] or [r, g, b, a]".to_string(),
                );
            }
            let r = get_int_arg(&list[0], "r")? as u8;
            let g = get_int_arg(&list[1], "g")? as u8;
            let b = get_int_arg(&list[2], "b")? as u8;
            let a = if list.len() > 3 {
                get_int_arg(&list[3], "a")? as u8
            } else {
                255
            };
            Ok(image::Rgba([r, g, b, a]))
        }
        _ => Err(format!(
            "color must be string or list, got {}",
            value.type_name()
        )),
    }
}

/// Methods on Image instances
pub fn image_method(img: &Arc<ImageWrapper>, method: &str, args: &[Value]) -> NativeResult {
    match method {
        // Basic info
        "width" => Ok(Value::Int(img.width() as i64)),
        "height" => Ok(Value::Int(img.height() as i64)),
        "dimensions" => {
            let (w, h) = img.dimensions();
            Ok(Value::list(vec![
                Value::Int(w as i64),
                Value::Int(h as i64),
            ]))
        }

        // Transformations
        "resize" => image_resize(img, args),
        "crop" => image_crop(img, args),
        "rotate" | "rotate90" => image_rotate(img, args),
        "flip_h" | "flip_horizontal" => image_flip_h(img),
        "flip_v" | "flip_vertical" => image_flip_v(img),

        // Color operations
        "grayscale" => image_grayscale(img),
        "invert" => image_invert(img),
        "brightness" => image_brightness(img, args),
        "contrast" => image_contrast(img, args),
        "hue_rotate" => image_hue_rotate(img, args),
        "saturate" => image_saturate(img, args),
        "blur" => image_blur(img, args),
        "sharpen" => image_sharpen(img),

        // I/O
        "save" => image_save(img, args),
        "to_bytes" => image_to_bytes(img, args),

        _ => Err(format!("Image has no method '{method}'")),
    }
}

/// img.resize(width: Int, height: Int) -> Image
fn image_resize(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(format!(
            "resize() expects 2 arguments (width, height), got {}",
            args.len()
        ));
    }
    let width = get_int_arg(&args[0], "width")? as u32;
    let height = get_int_arg(&args[1], "height")? as u32;

    let resized = img.image.resize(width, height, FilterType::Lanczos3);
    let wrapper = ImageWrapper::new(resized, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.crop(x: Int, y: Int, width: Int, height: Int) -> Image
fn image_crop(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    if args.len() != 4 {
        return Err(format!(
            "crop() expects 4 arguments (x, y, width, height), got {}",
            args.len()
        ));
    }
    let x = get_int_arg(&args[0], "x")? as u32;
    let y = get_int_arg(&args[1], "y")? as u32;
    let width = get_int_arg(&args[2], "width")? as u32;
    let height = get_int_arg(&args[3], "height")? as u32;

    let cropped = img.image.crop_imm(x, y, width, height);
    let wrapper = ImageWrapper::new(cropped, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.rotate(degrees: Int) -> Image
/// Rotate by 90, 180, or 270 degrees
fn image_rotate(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    let degrees = if args.is_empty() {
        90
    } else {
        get_int_arg(&args[0], "degrees")? as i32
    };

    let rotated = match degrees % 360 {
        90 | -270 => img.image.rotate90(),
        180 | -180 => img.image.rotate180(),
        270 | -90 => img.image.rotate270(),
        0 => (*img.image).clone(),
        _ => {
            return Err(format!(
                "rotate() only supports 90, 180, 270 degrees, got {}",
                degrees
            ))
        }
    };

    let wrapper = ImageWrapper::new(rotated, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.flip_h() -> Image
fn image_flip_h(img: &Arc<ImageWrapper>) -> NativeResult {
    let flipped = img.image.fliph();
    let wrapper = ImageWrapper::new(flipped, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.flip_v() -> Image
fn image_flip_v(img: &Arc<ImageWrapper>) -> NativeResult {
    let flipped = img.image.flipv();
    let wrapper = ImageWrapper::new(flipped, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.grayscale() -> Image
fn image_grayscale(img: &Arc<ImageWrapper>) -> NativeResult {
    let gray = img.image.grayscale();
    let wrapper = ImageWrapper::new(gray, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.invert() -> Image
fn image_invert(img: &Arc<ImageWrapper>) -> NativeResult {
    let mut inverted = (*img.image).clone();
    inverted.invert();
    let wrapper = ImageWrapper::new(inverted, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.brightness(value: Float) -> Image
/// Adjust brightness (-1.0 to 1.0, negative = darker, positive = lighter)
fn image_brightness(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "brightness() expects 1 argument, got {}",
            args.len()
        ));
    }
    let value = get_float_arg(&args[0], "value")?;
    // Clamp to -1.0 to 1.0
    let value = value.clamp(-1.0, 1.0);
    // Convert to i32 range (-255 to 255)
    let adj = (value * 255.0) as i32;

    let adjusted = img.image.brighten(adj);
    let wrapper = ImageWrapper::new(adjusted, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.contrast(value: Float) -> Image
/// Adjust contrast (0.0 = gray, 1.0 = no change, >1.0 = more contrast)
fn image_contrast(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("contrast() expects 1 argument, got {}", args.len()));
    }
    let value = get_float_arg(&args[0], "value")?;

    let adjusted = img.image.adjust_contrast(((value - 1.0) * 100.0) as f32);
    let wrapper = ImageWrapper::new(adjusted, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.hue_rotate(degrees: Int) -> Image
fn image_hue_rotate(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "hue_rotate() expects 1 argument, got {}",
            args.len()
        ));
    }
    let degrees = get_int_arg(&args[0], "degrees")? as i32;

    let rotated = img.image.huerotate(degrees);
    let wrapper = ImageWrapper::new(rotated, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.saturate(value: Float) -> Image
/// Adjust saturation (0.0 = grayscale, 1.0 = no change, >1.0 = more saturated)
fn image_saturate(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("saturate() expects 1 argument, got {}", args.len()));
    }
    let value = get_float_arg(&args[0], "value")?;

    // Use hue rotate + contrast as a simple saturation approximation
    // For proper saturation we'd need to convert to HSL
    // This is a simplified version
    if value <= 0.01 {
        // Fully desaturated - return grayscale
        let gray = img.image.grayscale();
        let wrapper = ImageWrapper::new(gray, img.source_path.clone(), img.format.clone());
        return Ok(Value::Image(Arc::new(wrapper)));
    }
    // For values >= 1.0, increase contrast slightly as approximation
    let contrast_adj = ((value - 1.0) * 20.0) as f32;
    let adjusted = img.image.adjust_contrast(contrast_adj);
    let wrapper = ImageWrapper::new(adjusted, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.blur(sigma: Float) -> Image
fn image_blur(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("blur() expects 1 argument, got {}", args.len()));
    }
    let sigma = get_float_arg(&args[0], "sigma")? as f32;

    let blurred = img.image.blur(sigma);
    let wrapper = ImageWrapper::new(blurred, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.sharpen() -> Image
fn image_sharpen(img: &Arc<ImageWrapper>) -> NativeResult {
    let sharpened = img.image.unsharpen(1.0, 1);
    let wrapper = ImageWrapper::new(sharpened, img.source_path.clone(), img.format.clone());
    Ok(Value::Image(Arc::new(wrapper)))
}

/// img.save(path: String) -> null
fn image_save(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "save() expects 1 argument (path), got {}",
            args.len()
        ));
    }
    let path = get_string_arg(&args[0], "path")?;

    img.image
        .save(&path)
        .map_err(|e| format!("failed to save image to '{}': {}", path, e))?;

    Ok(Value::Null)
}

/// img.to_bytes(format?: String) -> List<Int>
fn image_to_bytes(img: &Arc<ImageWrapper>, args: &[Value]) -> NativeResult {
    let format_str = if args.is_empty() {
        img.format.clone().unwrap_or_else(|| "png".to_string())
    } else {
        get_string_arg(&args[0], "format")?
    };

    let format = match format_str.to_lowercase().as_str() {
        "png" => ImageFormat::Png,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "gif" => ImageFormat::Gif,
        "bmp" => ImageFormat::Bmp,
        "webp" => ImageFormat::WebP,
        _ => return Err(format!("unsupported image format: {}", format_str)),
    };

    let mut bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    img.image
        .write_to(&mut cursor, format)
        .map_err(|e| format!("failed to encode image: {}", e))?;

    let values: Vec<Value> = bytes.iter().map(|&b| Value::Int(b as i64)).collect();
    Ok(Value::list(values))
}

/// Helper to get float argument
fn get_float_arg(value: &Value, name: &str) -> Result<f64, String> {
    match value {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        _ => Err(format!(
            "{} must be a number, got {}",
            name,
            value.type_name()
        )),
    }
}

// ============================================================================
// Ref Module - Weak References
// ============================================================================

pub fn ref_method(method: &str, args: &[Value]) -> NativeResult {
    match method {
        "weak" => ref_weak(args),
        "upgrade" => ref_upgrade(args),
        "is_alive" => ref_is_alive(args),
        _ => Err(format!("Ref has no method '{method}'")),
    }
}

fn ref_weak(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!("Ref.weak() expects 1 argument, got {}", args.len()));
    }
    match args[0].weak_ref() {
        Some(weak) => Ok(weak),
        None => Err(format!(
            "Ref.weak() requires a container type (List, Map, Set, or Struct), got {}",
            args[0].type_name()
        )),
    }
}

fn ref_upgrade(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Ref.upgrade() expects 1 argument, got {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::WeakRef(weak) => Ok(weak.upgrade().unwrap_or(Value::Null)),
        _ => Err(format!(
            "Ref.upgrade() requires a WeakRef, got {}",
            args[0].type_name()
        )),
    }
}

fn ref_is_alive(args: &[Value]) -> NativeResult {
    if args.len() != 1 {
        return Err(format!(
            "Ref.is_alive() expects 1 argument, got {}",
            args.len()
        ));
    }
    match &args[0] {
        Value::WeakRef(weak) => Ok(Value::Bool(weak.is_alive())),
        _ => Err(format!(
            "Ref.is_alive() requires a WeakRef, got {}",
            args[0].type_name()
        )),
    }
}

/// Methods available directly on WeakRef values
pub fn weak_ref_method(method: &str, args: &[Value], weak: &WeakRefValue) -> NativeResult {
    match method {
        "upgrade" => {
            if !args.is_empty() {
                return Err(format!("upgrade() expects 0 arguments, got {}", args.len()));
            }
            Ok(weak.upgrade().unwrap_or(Value::Null))
        }
        "is_alive" => {
            if !args.is_empty() {
                return Err(format!(
                    "is_alive() expects 0 arguments, got {}",
                    args.len()
                ));
            }
            Ok(Value::Bool(weak.is_alive()))
        }
        "target_type" => {
            if !args.is_empty() {
                return Err(format!(
                    "target_type() expects 0 arguments, got {}",
                    args.len()
                ));
            }
            Ok(Value::string(weak.target_type_name()))
        }
        _ => Err(format!("WeakRef has no method '{method}'")),
    }
}

/// Dispatch a method call on a native namespace
pub fn dispatch_namespace_method(namespace: &str, method: &str, args: &[Value]) -> NativeResult {
    match namespace {
        "Set" => set_native_method(method, args),
        "File" => file_method(method, args),
        "Dir" => dir_method(method, args),
        "Path" => path_method(method, args),
        "Env" => env_method(method, args),
        "Args" => args_method(method, args),
        "Shell" => shell_method(method, args),
        "Http" => http_method(method, args),
        "Json" => json_method(method, args),
        "Toml" => toml_method(method, args),
        "Yaml" => yaml_method(method, args),
        "Base64" => base64_method(method, args),
        "Url" => url_method(method, args),
        "Gzip" => gzip_method(method, args),
        "Zip" => zip_method(method, args),
        "DateTime" => datetime_method(method, args),
        "Duration" => duration_method(method, args),
        "Time" => time_method(method, args),
        "Regex" => regex_method(method, args),
        "Hash" => hash_method(method, args),
        "Crypto" => crypto_method(method, args),
        "Uuid" => uuid_method(method, args),
        "Random" => random_method(method, args),
        "Math" => math_method(method, args),
        "Input" => input_method(method, args),
        "Log" => log_method(method, args),
        "System" => system_method(method, args),
        "Process" => process_method(method, args),
        "Signal" => signal_method(method, args),
        "Db" => db_method(method, args),
        "Async" => async_method(method, args),
        "Tcp" => tcp_method(method, args),
        "Udp" => udp_method(method, args),
        "WebSocket" => ws_method(method, args),
        "Data" => data_method(method, args),
        "Agg" => agg_method(method, args),
        "Join" => join_method(method, args),
        "Cube" => cube_method(method, args),
        "Test" => test_method(method, args),
        "Xml" => xml_method(method, args),
        "Image" => image_namespace_method(method, args),
        "Ref" => ref_method(method, args),
        _ => Err(format!("unknown namespace '{}'", namespace)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ============================================================================
    // File Module Tests
    // ============================================================================

    #[test]
    fn test_file_read_write_text() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let path_str = path.to_string_lossy().to_string();

        // Write
        let result = file_method(
            "write_text",
            &[Value::string(&path_str), Value::string("Hello, World!")],
        );
        assert!(result.is_ok());

        // Read
        let result = file_method("read_text", &[Value::string(&path_str)]);
        assert!(matches!(result, Ok(Value::String(s)) if *s == "Hello, World!"));

        // Exists
        let result = file_method("exists", &[Value::string(&path_str)]);
        assert_eq!(result, Ok(Value::Bool(true)));

        // Size
        let result = file_method("size", &[Value::string(&path_str)]);
        assert_eq!(result, Ok(Value::Int(13)));
    }

    #[test]
    fn test_file_read_lines() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lines.txt");
        let path_str = path.to_string_lossy().to_string();

        fs::write(&path, "line1\nline2\nline3").unwrap();

        let result = file_method("read_lines", &[Value::string(&path_str)]).unwrap();
        if let Value::List(list) = result {
            let list = list.borrow();
            assert_eq!(list.len(), 3);
            assert_eq!(list[0], Value::string("line1"));
            assert_eq!(list[1], Value::string("line2"));
            assert_eq!(list[2], Value::string("line3"));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_file_append() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("append.txt");
        let path_str = path.to_string_lossy().to_string();

        file_method(
            "write_text",
            &[Value::string(&path_str), Value::string("Hello")],
        )
        .unwrap();
        file_method(
            "append",
            &[Value::string(&path_str), Value::string(", World!")],
        )
        .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_file_delete() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("delete.txt");
        let path_str = path.to_string_lossy().to_string();

        fs::write(&path, "test").unwrap();
        assert!(path.exists());

        file_method("delete", &[Value::string(&path_str)]).unwrap();
        assert!(!path.exists());
    }

    // ============================================================================
    // Dir Module Tests
    // ============================================================================

    #[test]
    fn test_dir_create_and_list() {
        let dir = tempdir().unwrap();
        let new_dir = dir.path().join("subdir");
        let dir_str = new_dir.to_string_lossy().to_string();

        // Create
        dir_method("create", &[Value::string(&dir_str)]).unwrap();
        assert!(new_dir.is_dir());

        // Create files in dir
        fs::write(new_dir.join("a.txt"), "a").unwrap();
        fs::write(new_dir.join("b.txt"), "b").unwrap();

        // List
        let result = dir_method("list", &[Value::string(&dir_str)]).unwrap();
        if let Value::List(list) = result {
            let list = list.borrow();
            assert_eq!(list.len(), 2);
        } else {
            panic!("Expected List");
        }

        // Exists
        let result = dir_method("exists", &[Value::string(&dir_str)]);
        assert_eq!(result, Ok(Value::Bool(true)));
    }

    #[test]
    fn test_dir_create_all() {
        let dir = tempdir().unwrap();
        let deep_path = dir.path().join("a/b/c");
        let path_str = deep_path.to_string_lossy().to_string();

        dir_method("create_all", &[Value::string(&path_str)]).unwrap();
        assert!(deep_path.is_dir());
    }

    // ============================================================================
    // Path Module Tests
    // ============================================================================

    #[test]
    fn test_path_join() {
        let result = path_method(
            "join",
            &[
                Value::string("/home"),
                Value::string("user"),
                Value::string("file.txt"),
            ],
        )
        .unwrap();

        if let Value::String(s) = result {
            assert!(s.contains("home") && s.contains("user") && s.contains("file.txt"));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_path_extension() {
        let result = path_method("extension", &[Value::string("/path/to/file.txt")]).unwrap();
        assert_eq!(result, Value::string("txt"));

        let result = path_method("extension", &[Value::string("/path/to/file")]).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_path_filename() {
        let result = path_method("filename", &[Value::string("/path/to/file.txt")]).unwrap();
        assert_eq!(result, Value::string("file.txt"));
    }

    #[test]
    fn test_path_parent() {
        let result = path_method("parent", &[Value::string("/path/to/file.txt")]).unwrap();
        if let Value::String(s) = result {
            assert!(s.ends_with("to") || s.ends_with("to/"));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_path_stem() {
        let result = path_method("stem", &[Value::string("/path/to/file.txt")]).unwrap();
        assert_eq!(result, Value::string("file"));
    }

    #[test]
    fn test_path_is_absolute() {
        let result = path_method("is_absolute", &[Value::string("/absolute/path")]).unwrap();
        assert_eq!(result, Value::Bool(true));

        let result = path_method("is_absolute", &[Value::string("relative/path")]).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    // ============================================================================
    // Env Module Tests
    // ============================================================================

    #[test]
    fn test_env_get_set() {
        let test_var = "STRATUM_TEST_VAR_12345";

        // Get nonexistent
        let result = env_method("get", &[Value::string(test_var)]).unwrap();
        assert_eq!(result, Value::Null);

        // Get with default
        let result =
            env_method("get", &[Value::string(test_var), Value::string("default")]).unwrap();
        assert_eq!(result, Value::string("default"));

        // Set
        env_method(
            "set",
            &[Value::string(test_var), Value::string("test_value")],
        )
        .unwrap();

        // Get again
        let result = env_method("get", &[Value::string(test_var)]).unwrap();
        assert_eq!(result, Value::string("test_value"));

        // Has
        let result = env_method("has", &[Value::string(test_var)]).unwrap();
        assert_eq!(result, Value::Bool(true));

        // Remove
        env_method("remove", &[Value::string(test_var)]).unwrap();

        // Verify removed
        let result = env_method("has", &[Value::string(test_var)]).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_env_all() {
        let result = env_method("all", &[]).unwrap();
        if let Value::Map(map) = result {
            // Should have at least PATH or similar
            assert!(!map.borrow().is_empty());
        } else {
            panic!("Expected Map");
        }
    }

    // ============================================================================
    // Args Module Tests
    // ============================================================================

    #[test]
    fn test_args_all() {
        let result = args_method("all", &[]).unwrap();
        if let Value::List(list) = result {
            // First arg should be the test binary
            assert!(!list.borrow().is_empty());
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_args_count() {
        let result = args_method("count", &[]).unwrap();
        if let Value::Int(count) = result {
            assert!(count >= 1); // At least the binary name
        } else {
            panic!("Expected Int");
        }
    }

    // ============================================================================
    // Shell Module Tests
    // ============================================================================

    #[test]
    fn test_shell_exec() {
        let result = shell_method("exec", &[Value::string("echo hello")]).unwrap();
        if let Value::String(s) = result {
            assert_eq!(*s, "hello");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_shell_run() {
        let result = shell_method(
            "run",
            &[
                Value::string("echo"),
                Value::list(vec![Value::string("hello"), Value::string("world")]),
            ],
        )
        .unwrap();

        if let Value::Map(map) = result {
            let map = map.borrow();
            let stdout_key = HashableValue::String(Rc::new("stdout".to_string()));
            if let Some(Value::String(stdout)) = map.get(&stdout_key) {
                assert!(stdout.contains("hello"));
            }
            let exit_key = HashableValue::String(Rc::new("exit_code".to_string()));
            if let Some(Value::Int(code)) = map.get(&exit_key) {
                assert_eq!(*code, 0);
            }
        } else {
            panic!("Expected Map");
        }
    }

    // ============================================================================
    // Http Module Tests
    // ============================================================================

    #[test]
    fn test_http_get_missing_args() {
        let result = http_method("get", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1-2 arguments"));
    }

    #[test]
    fn test_http_get_invalid_url_type() {
        let result = http_method("get", &[Value::Int(123)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be String"));
    }

    #[test]
    fn test_http_post_missing_args() {
        let result = http_method("post", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1-3 arguments"));
    }

    #[test]
    fn test_http_unknown_method() {
        let result = http_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    #[test]
    fn test_http_options_extraction() {
        // Test with invalid options type
        let result = extract_http_options(&Value::Int(123));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be Map"));

        // Test with valid empty options
        let empty_map = Value::Map(Rc::new(RefCell::new(HashMap::new())));
        let (headers, timeout) = extract_http_options(&empty_map).unwrap();
        assert!(headers.is_empty());
        assert!(timeout.is_none());

        // Test with timeout option
        let mut map = HashMap::new();
        map.insert(
            HashableValue::String(Rc::new("timeout".to_string())),
            Value::Int(5000),
        );
        let options_map = Value::Map(Rc::new(RefCell::new(map)));
        let (headers, timeout) = extract_http_options(&options_map).unwrap();
        assert!(headers.is_empty());
        assert_eq!(timeout, Some(5000));

        // Test with headers option
        let mut headers_map = HashMap::new();
        headers_map.insert(
            HashableValue::String(Rc::new("Content-Type".to_string())),
            Value::string("application/json"),
        );
        let mut map = HashMap::new();
        map.insert(
            HashableValue::String(Rc::new("headers".to_string())),
            Value::Map(Rc::new(RefCell::new(headers_map))),
        );
        let options_map = Value::Map(Rc::new(RefCell::new(map)));
        let (headers, timeout) = extract_http_options(&options_map).unwrap();
        assert_eq!(
            headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert!(timeout.is_none());
    }

    #[test]
    fn test_http_get_invalid_url() {
        // Invalid URL should return an error
        let result = http_method("get", &[Value::string("not-a-valid-url")]);
        assert!(result.is_err());
    }

    #[test]
    fn test_http_connection_refused() {
        // Attempting to connect to a closed port should return an error
        let result = http_method("get", &[Value::string("http://127.0.0.1:1")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("request failed"));
    }

    // Integration test - requires network access
    // Uses httpbin.org which is a testing service for HTTP clients
    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_http_get_real_request() {
        let result = http_method("get", &[Value::string("https://httpbin.org/get")]);
        assert!(result.is_ok());

        if let Ok(Value::Map(map)) = result {
            let map = map.borrow();

            // Check status
            let status_key = HashableValue::String(Rc::new("status".to_string()));
            if let Some(Value::Int(status)) = map.get(&status_key) {
                assert_eq!(*status, 200);
            } else {
                panic!("Expected status Int");
            }

            // Check ok
            let ok_key = HashableValue::String(Rc::new("ok".to_string()));
            if let Some(Value::Bool(ok)) = map.get(&ok_key) {
                assert!(*ok);
            } else {
                panic!("Expected ok Bool");
            }

            // Check body is non-empty
            let body_key = HashableValue::String(Rc::new("body".to_string()));
            if let Some(Value::String(body)) = map.get(&body_key) {
                assert!(!body.is_empty());
            } else {
                panic!("Expected body String");
            }
        } else {
            panic!("Expected Map result");
        }
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_http_post_real_request() {
        let result = http_method(
            "post",
            &[
                Value::string("https://httpbin.org/post"),
                Value::string("{\"test\": true}"),
            ],
        );
        assert!(result.is_ok());

        if let Ok(Value::Map(map)) = result {
            let map = map.borrow();
            let status_key = HashableValue::String(Rc::new("status".to_string()));
            if let Some(Value::Int(status)) = map.get(&status_key) {
                assert_eq!(*status, 200);
            }
        }
    }

    // ============================================================================
    // Dispatch Tests
    // ============================================================================

    #[test]
    fn test_dispatch_unknown_namespace() {
        let result = dispatch_namespace_method("Unknown", "method", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_dispatch_unknown_method() {
        let result = dispatch_namespace_method("File", "unknown_method", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_dispatch_http_namespace() {
        // Verify Http is properly routed through dispatch
        let result = dispatch_namespace_method("Http", "get", &[]);
        assert!(result.is_err()); // Should fail due to missing args, but proves routing works
        assert!(result.unwrap_err().contains("expects 1-2 arguments"));
    }

    // ============================================================================
    // Json Module Tests
    // ============================================================================

    #[test]
    fn test_json_encode_primitives() {
        // Null
        let result = json_method("encode", &[Value::Null]).unwrap();
        assert_eq!(result, Value::string("null"));

        // Bool
        let result = json_method("encode", &[Value::Bool(true)]).unwrap();
        assert_eq!(result, Value::string("true"));

        // Int
        let result = json_method("encode", &[Value::Int(42)]).unwrap();
        assert_eq!(result, Value::string("42"));

        // Float
        let result = json_method("encode", &[Value::Float(3.14)]).unwrap();
        assert_eq!(result, Value::string("3.14"));

        // String
        let result = json_method("encode", &[Value::string("hello")]).unwrap();
        assert_eq!(result, Value::string("\"hello\""));
    }

    #[test]
    fn test_json_encode_list() {
        let list = Value::list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let result = json_method("encode", &[list]).unwrap();
        assert_eq!(result, Value::string("[1,2,3]"));
    }

    #[test]
    fn test_json_encode_map() {
        let mut map = HashMap::new();
        map.insert(
            HashableValue::String(Rc::new("name".to_string())),
            Value::string("test"),
        );
        map.insert(
            HashableValue::String(Rc::new("value".to_string())),
            Value::Int(42),
        );
        let map_value = Value::Map(Rc::new(RefCell::new(map)));

        let result = json_method("encode", &[map_value]).unwrap();
        if let Value::String(s) = result {
            // Order may vary in HashMap, so check both keys exist
            assert!(s.contains("\"name\":\"test\"") || s.contains("\"name\": \"test\""));
            assert!(s.contains("\"value\":42") || s.contains("\"value\": 42"));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_json_decode_primitives() {
        // Null
        let result = json_method("decode", &[Value::string("null")]).unwrap();
        assert_eq!(result, Value::Null);

        // Bool
        let result = json_method("decode", &[Value::string("true")]).unwrap();
        assert_eq!(result, Value::Bool(true));

        // Int
        let result = json_method("decode", &[Value::string("42")]).unwrap();
        assert_eq!(result, Value::Int(42));

        // Float
        let result = json_method("decode", &[Value::string("3.14")]).unwrap();
        assert_eq!(result, Value::Float(3.14));

        // String
        let result = json_method("decode", &[Value::string("\"hello\"")]).unwrap();
        assert_eq!(result, Value::string("hello"));
    }

    #[test]
    fn test_json_decode_array() {
        let result = json_method("decode", &[Value::string("[1, 2, 3]")]).unwrap();
        if let Value::List(list) = result {
            let list = list.borrow();
            assert_eq!(list.len(), 3);
            assert_eq!(list[0], Value::Int(1));
            assert_eq!(list[1], Value::Int(2));
            assert_eq!(list[2], Value::Int(3));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_json_decode_object() {
        let result = json_method(
            "decode",
            &[Value::string("{\"name\": \"test\", \"value\": 42}")],
        )
        .unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let name_key = HashableValue::String(Rc::new("name".to_string()));
            let value_key = HashableValue::String(Rc::new("value".to_string()));
            assert_eq!(map.get(&name_key), Some(&Value::string("test")));
            assert_eq!(map.get(&value_key), Some(&Value::Int(42)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_json_roundtrip() {
        let original = Value::list(vec![
            Value::Int(1),
            Value::string("hello"),
            Value::Bool(true),
            Value::Null,
        ]);
        let encoded = json_method("encode", &[original.clone()]).unwrap();
        let decoded = json_method("decode", &[encoded]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_json_decode_invalid() {
        let result = json_method("decode", &[Value::string("invalid json")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed to parse JSON"));
    }

    #[test]
    fn test_json_wrong_args() {
        // Missing args
        let result = json_method("encode", &[]);
        assert!(result.is_err());

        // Too many args
        let result = json_method("encode", &[Value::Int(1), Value::Int(2)]);
        assert!(result.is_err());
    }

    // ============================================================================
    // Toml Module Tests
    // ============================================================================

    #[test]
    fn test_toml_encode_primitives() {
        // Note: TOML requires a table structure at the root for encoding
        let mut map = HashMap::new();
        map.insert(
            HashableValue::String(Rc::new("value".to_string())),
            Value::Int(42),
        );
        let map_value = Value::Map(Rc::new(RefCell::new(map)));

        let result = toml_method("encode", &[map_value]).unwrap();
        if let Value::String(s) = result {
            assert!(s.contains("value = 42"));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_toml_decode() {
        let toml_str = r#"
            name = "test"
            value = 42
            enabled = true
        "#;
        let result = toml_method("decode", &[Value::string(toml_str)]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let name_key = HashableValue::String(Rc::new("name".to_string()));
            let value_key = HashableValue::String(Rc::new("value".to_string()));
            let enabled_key = HashableValue::String(Rc::new("enabled".to_string()));
            assert_eq!(map.get(&name_key), Some(&Value::string("test")));
            assert_eq!(map.get(&value_key), Some(&Value::Int(42)));
            assert_eq!(map.get(&enabled_key), Some(&Value::Bool(true)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_toml_nested() {
        let toml_str = r#"
            [server]
            host = "localhost"
            port = 8080
        "#;
        let result = toml_method("decode", &[Value::string(toml_str)]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let server_key = HashableValue::String(Rc::new("server".to_string()));
            if let Some(Value::Map(server)) = map.get(&server_key) {
                let server = server.borrow();
                let host_key = HashableValue::String(Rc::new("host".to_string()));
                let port_key = HashableValue::String(Rc::new("port".to_string()));
                assert_eq!(server.get(&host_key), Some(&Value::string("localhost")));
                assert_eq!(server.get(&port_key), Some(&Value::Int(8080)));
            } else {
                panic!("Expected server to be Map");
            }
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_toml_decode_invalid() {
        let result = toml_method("decode", &[Value::string("[invalid")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed to parse TOML"));
    }

    #[test]
    fn test_toml_null_not_supported() {
        let result = toml_method("encode", &[Value::Null]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not support null"));
    }

    // ============================================================================
    // Yaml Module Tests
    // ============================================================================

    #[test]
    fn test_yaml_encode_primitives() {
        // Null
        let result = yaml_method("encode", &[Value::Null]).unwrap();
        if let Value::String(s) = result {
            assert!(s.contains("null") || s.trim() == "~");
        } else {
            panic!("Expected String");
        }

        // Int
        let result = yaml_method("encode", &[Value::Int(42)]).unwrap();
        if let Value::String(s) = result {
            assert!(s.contains("42"));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_yaml_decode() {
        let yaml_str = r#"
            name: test
            value: 42
            enabled: true
        "#;
        let result = yaml_method("decode", &[Value::string(yaml_str)]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let name_key = HashableValue::String(Rc::new("name".to_string()));
            let value_key = HashableValue::String(Rc::new("value".to_string()));
            let enabled_key = HashableValue::String(Rc::new("enabled".to_string()));
            assert_eq!(map.get(&name_key), Some(&Value::string("test")));
            assert_eq!(map.get(&value_key), Some(&Value::Int(42)));
            assert_eq!(map.get(&enabled_key), Some(&Value::Bool(true)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_yaml_list() {
        let yaml_str = "- 1\n- 2\n- 3";
        let result = yaml_method("decode", &[Value::string(yaml_str)]).unwrap();
        if let Value::List(list) = result {
            let list = list.borrow();
            assert_eq!(list.len(), 3);
            assert_eq!(list[0], Value::Int(1));
            assert_eq!(list[1], Value::Int(2));
            assert_eq!(list[2], Value::Int(3));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_yaml_decode_invalid() {
        let result = yaml_method("decode", &[Value::string("key: [unclosed")]);
        assert!(result.is_err());
    }

    // ============================================================================
    // Base64 Module Tests
    // ============================================================================

    #[test]
    fn test_base64_encode_string() {
        let result = base64_method("encode", &[Value::string("Hello, World!")]).unwrap();
        assert_eq!(result, Value::string("SGVsbG8sIFdvcmxkIQ=="));
    }

    #[test]
    fn test_base64_encode_bytes() {
        let bytes = Value::list(vec![Value::Int(72), Value::Int(105)]); // "Hi"
        let result = base64_method("encode", &[bytes]).unwrap();
        assert_eq!(result, Value::string("SGk="));
    }

    #[test]
    fn test_base64_decode_string() {
        let result = base64_method("decode", &[Value::string("SGVsbG8sIFdvcmxkIQ==")]).unwrap();
        assert_eq!(result, Value::string("Hello, World!"));
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = Value::string("Test string for roundtrip");
        let encoded = base64_method("encode", &[original.clone()]).unwrap();
        let decoded = base64_method("decode", &[encoded]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_base64_decode_invalid() {
        let result = base64_method("decode", &[Value::string("!!invalid!!")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed to decode base64"));
    }

    #[test]
    fn test_base64_wrong_args() {
        let result = base64_method("encode", &[]);
        assert!(result.is_err());

        let result = base64_method("encode", &[Value::Int(123)]);
        assert!(result.is_err());
    }

    // ============================================================================
    // Url Module Tests
    // ============================================================================

    #[test]
    fn test_url_encode() {
        let result = url_method("encode", &[Value::string("hello world")]).unwrap();
        assert_eq!(result, Value::string("hello%20world"));
    }

    #[test]
    fn test_url_encode_special_chars() {
        let result = url_method("encode", &[Value::string("foo=bar&baz=qux")]).unwrap();
        if let Value::String(s) = result {
            assert!(s.contains("%3D")); // =
            assert!(s.contains("%26")); // &
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_url_decode() {
        let result = url_method("decode", &[Value::string("hello%20world")]).unwrap();
        assert_eq!(result, Value::string("hello world"));
    }

    #[test]
    fn test_url_roundtrip() {
        let original = Value::string("Hello World! Special: &=?#");
        let encoded = url_method("encode", &[original.clone()]).unwrap();
        let decoded = url_method("decode", &[encoded]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_url_decode_invalid_utf8() {
        // Invalid UTF-8 sequence after decoding
        let result = url_method("decode", &[Value::string("%FF%FE")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed to decode URL"));
    }

    #[test]
    fn test_url_wrong_args() {
        let result = url_method("encode", &[]);
        assert!(result.is_err());

        let result = url_method("encode", &[Value::Int(123)]);
        assert!(result.is_err());
    }

    // ============================================================================
    // Dispatch Tests for Encoding Modules
    // ============================================================================

    #[test]
    fn test_dispatch_json_namespace() {
        let result = dispatch_namespace_method("Json", "encode", &[Value::Int(42)]).unwrap();
        assert_eq!(result, Value::string("42"));
    }

    #[test]
    fn test_dispatch_toml_namespace() {
        let result =
            dispatch_namespace_method("Toml", "decode", &[Value::string("key = \"value\"")])
                .unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new("key".to_string()));
            assert_eq!(map.get(&key), Some(&Value::string("value")));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_dispatch_yaml_namespace() {
        let result =
            dispatch_namespace_method("Yaml", "decode", &[Value::string("key: value")]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new("key".to_string()));
            assert_eq!(map.get(&key), Some(&Value::string("value")));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_dispatch_base64_namespace() {
        let result =
            dispatch_namespace_method("Base64", "encode", &[Value::string("test")]).unwrap();
        assert_eq!(result, Value::string("dGVzdA=="));
    }

    #[test]
    fn test_dispatch_url_namespace() {
        let result = dispatch_namespace_method("Url", "encode", &[Value::string("a b")]).unwrap();
        assert_eq!(result, Value::string("a%20b"));
    }

    // ============================================================================
    // Gzip Module Tests
    // ============================================================================

    #[test]
    fn test_gzip_compress_decompress() {
        // Create test bytes
        let test_bytes: Vec<Value> = b"Hello, World!"
            .iter()
            .map(|b| Value::Int(i64::from(*b)))
            .collect();
        let input = Value::list(test_bytes.clone());

        // Compress
        let compressed = gzip_method("compress", &[input]).unwrap();

        // Decompress
        let decompressed = gzip_method("decompress", &[compressed]).unwrap();

        // Verify
        if let Value::List(list) = decompressed {
            let bytes: Vec<u8> = list
                .borrow()
                .iter()
                .map(|v| match v {
                    Value::Int(i) => *i as u8,
                    _ => panic!("Expected Int"),
                })
                .collect();
            assert_eq!(bytes, b"Hello, World!");
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_gzip_compress_text_decompress_text() {
        let input = Value::string("Hello, Stratum!");

        // Compress text
        let compressed = gzip_method("compress_text", &[input]).unwrap();

        // Decompress to text
        let decompressed = gzip_method("decompress_text", &[compressed]).unwrap();

        assert_eq!(decompressed, Value::string("Hello, Stratum!"));
    }

    // ============================================================================
    // Zip Module Tests
    // ============================================================================

    #[test]
    fn test_zip_create_and_list() {
        let dir = tempdir().unwrap();

        // Create test files
        let file1_path = dir.path().join("file1.txt");
        let file2_path = dir.path().join("file2.txt");
        fs::write(&file1_path, "Content 1").unwrap();
        fs::write(&file2_path, "Content 2").unwrap();

        // Create zip
        let zip_path = dir.path().join("test.zip");
        let zip_path_str = zip_path.to_string_lossy().to_string();
        let files = Value::list(vec![
            Value::string(file1_path.to_string_lossy()),
            Value::string(file2_path.to_string_lossy()),
        ]);

        let result = zip_method("create", &[Value::string(&zip_path_str), files]).unwrap();
        assert_eq!(result, Value::Null);
        assert!(zip_path.exists());

        // List contents
        let entries = zip_method("list", &[Value::string(&zip_path_str)]).unwrap();
        if let Value::List(list) = entries {
            let list = list.borrow();
            assert_eq!(list.len(), 2);

            // Check that both files are present
            let names: Vec<String> = list
                .iter()
                .filter_map(|v| {
                    if let Value::Map(map) = v {
                        let map = map.borrow();
                        let key = HashableValue::String(Rc::new("name".to_string()));
                        map.get(&key).and_then(|v| {
                            if let Value::String(s) = v {
                                Some(s.to_string())
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                })
                .collect();
            assert!(names.contains(&"file1.txt".to_string()));
            assert!(names.contains(&"file2.txt".to_string()));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_zip_extract() {
        let dir = tempdir().unwrap();

        // Create a test file and zip it
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Test content").unwrap();

        let zip_path = dir.path().join("test.zip");
        let zip_path_str = zip_path.to_string_lossy().to_string();
        let files = Value::list(vec![Value::string(file_path.to_string_lossy())]);

        zip_method("create", &[Value::string(&zip_path_str), files]).unwrap();

        // Extract to new directory
        let extract_dir = dir.path().join("extracted");
        let extract_dir_str = extract_dir.to_string_lossy().to_string();

        let result = zip_method(
            "extract",
            &[
                Value::string(&zip_path_str),
                Value::string(&extract_dir_str),
            ],
        )
        .unwrap();
        assert_eq!(result, Value::Null);

        // Verify extracted file
        let extracted_file = extract_dir.join("test.txt");
        assert!(extracted_file.exists());
        assert_eq!(fs::read_to_string(extracted_file).unwrap(), "Test content");
    }

    #[test]
    fn test_zip_read_text() {
        let dir = tempdir().unwrap();

        // Create a test file and zip it
        let file_path = dir.path().join("readme.txt");
        fs::write(&file_path, "Hello from zip!").unwrap();

        let zip_path = dir.path().join("test.zip");
        let zip_path_str = zip_path.to_string_lossy().to_string();
        let files = Value::list(vec![Value::string(file_path.to_string_lossy())]);

        zip_method("create", &[Value::string(&zip_path_str), files]).unwrap();

        // Read file directly from zip
        let content = zip_method(
            "read_text",
            &[Value::string(&zip_path_str), Value::string("readme.txt")],
        )
        .unwrap();
        assert_eq!(content, Value::string("Hello from zip!"));
    }

    // ============================================================================
    // DateTime Module Tests
    // ============================================================================

    #[test]
    fn test_datetime_now() {
        let result = datetime_method("now", &[]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            // Should have all datetime fields
            let year_key = HashableValue::String(Rc::new("year".to_string()));
            let month_key = HashableValue::String(Rc::new("month".to_string()));
            let day_key = HashableValue::String(Rc::new("day".to_string()));
            let timestamp_key = HashableValue::String(Rc::new("timestamp".to_string()));

            assert!(map.contains_key(&year_key));
            assert!(map.contains_key(&month_key));
            assert!(map.contains_key(&day_key));
            assert!(map.contains_key(&timestamp_key));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_datetime_parse_iso8601() {
        let result = datetime_method("parse", &[Value::string("2025-01-15T10:30:00Z")]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let year_key = HashableValue::String(Rc::new("year".to_string()));
            let month_key = HashableValue::String(Rc::new("month".to_string()));
            let day_key = HashableValue::String(Rc::new("day".to_string()));
            let hour_key = HashableValue::String(Rc::new("hour".to_string()));

            assert_eq!(map.get(&year_key), Some(&Value::Int(2025)));
            assert_eq!(map.get(&month_key), Some(&Value::Int(1)));
            assert_eq!(map.get(&day_key), Some(&Value::Int(15)));
            assert_eq!(map.get(&hour_key), Some(&Value::Int(10)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_datetime_parse_with_format() {
        let result = datetime_method(
            "parse",
            &[
                Value::string("2025-01-15 14:30:00"),
                Value::string("%Y-%m-%d %H:%M:%S"),
            ],
        )
        .unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let year_key = HashableValue::String(Rc::new("year".to_string()));
            assert_eq!(map.get(&year_key), Some(&Value::Int(2025)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_datetime_from_timestamp() {
        // 2025-01-15 00:00:00 UTC in milliseconds
        let ts = 1736899200000_i64;
        let result = datetime_method("from_timestamp", &[Value::Int(ts)]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let year_key = HashableValue::String(Rc::new("year".to_string()));
            let timestamp_key = HashableValue::String(Rc::new("timestamp".to_string()));

            assert_eq!(map.get(&year_key), Some(&Value::Int(2025)));
            assert_eq!(map.get(&timestamp_key), Some(&Value::Int(ts)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_datetime_format() {
        let ts = 1736899200000_i64;
        let dt = datetime_method("from_timestamp", &[Value::Int(ts)]).unwrap();
        let result = datetime_method("format", &[dt, Value::string("%Y-%m-%d")]).unwrap();
        assert_eq!(result, Value::string("2025-01-15"));
    }

    #[test]
    fn test_datetime_components() {
        let ts = 1736944200000_i64; // 2025-01-15 12:30:00 UTC
        let dt = datetime_method("from_timestamp", &[Value::Int(ts)]).unwrap();

        assert_eq!(
            datetime_method("year", &[dt.clone()]).unwrap(),
            Value::Int(2025)
        );
        assert_eq!(
            datetime_method("month", &[dt.clone()]).unwrap(),
            Value::Int(1)
        );
        assert_eq!(
            datetime_method("day", &[dt.clone()]).unwrap(),
            Value::Int(15)
        );
        assert_eq!(
            datetime_method("hour", &[dt.clone()]).unwrap(),
            Value::Int(12)
        );
        assert_eq!(
            datetime_method("minute", &[dt.clone()]).unwrap(),
            Value::Int(30)
        );
        assert_eq!(datetime_method("second", &[dt]).unwrap(), Value::Int(0));
    }

    #[test]
    fn test_datetime_weekday() {
        // 2025-01-15 is a Wednesday
        let ts = 1736899200000_i64;
        let dt = datetime_method("from_timestamp", &[Value::Int(ts)]).unwrap();
        let result = datetime_method("weekday", &[dt]).unwrap();
        assert_eq!(result, Value::string("Wednesday"));
    }

    #[test]
    fn test_datetime_add_subtract() {
        let ts = 1736899200000_i64;
        let dt = datetime_method("from_timestamp", &[Value::Int(ts)]).unwrap();
        let one_day = duration_method("days", &[Value::Int(1)]).unwrap();

        // Add one day
        let result = datetime_method("add", &[dt.clone(), one_day.clone()]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let day_key = HashableValue::String(Rc::new("day".to_string()));
            assert_eq!(map.get(&day_key), Some(&Value::Int(16)));
        } else {
            panic!("Expected Map");
        }

        // Subtract one day (should get back original)
        let added_dt = datetime_method("add", &[dt.clone(), one_day.clone()]).unwrap();
        let result = datetime_method("subtract", &[added_dt, one_day]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let day_key = HashableValue::String(Rc::new("day".to_string()));
            assert_eq!(map.get(&day_key), Some(&Value::Int(15)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_datetime_diff() {
        let ts1 = 1736899200000_i64; // 2025-01-15
        let ts2 = 1736985600000_i64; // 2025-01-16 (1 day later)
        let dt1 = datetime_method("from_timestamp", &[Value::Int(ts1)]).unwrap();
        let dt2 = datetime_method("from_timestamp", &[Value::Int(ts2)]).unwrap();

        let diff = datetime_method("diff", &[dt2, dt1]).unwrap();
        let millis = duration_method("as_millis", &[diff]).unwrap();
        assert_eq!(millis, Value::Int(86_400_000)); // 1 day in millis
    }

    #[test]
    fn test_datetime_compare() {
        let ts1 = 1736899200000_i64;
        let ts2 = 1736985600000_i64;
        let dt1 = datetime_method("from_timestamp", &[Value::Int(ts1)]).unwrap();
        let dt2 = datetime_method("from_timestamp", &[Value::Int(ts2)]).unwrap();

        assert_eq!(
            datetime_method("compare", &[dt1.clone(), dt2.clone()]).unwrap(),
            Value::Int(-1)
        );
        assert_eq!(
            datetime_method("compare", &[dt2.clone(), dt1.clone()]).unwrap(),
            Value::Int(1)
        );
        assert_eq!(
            datetime_method("compare", &[dt1.clone(), dt1]).unwrap(),
            Value::Int(0)
        );
    }

    #[test]
    fn test_datetime_to_timezone() {
        let ts = 1736899200000_i64; // 2025-01-15 00:00:00 UTC
        let dt = datetime_method("from_timestamp", &[Value::Int(ts)]).unwrap();

        // Convert to New York timezone (UTC-5 in January)
        let result =
            datetime_method("to_timezone", &[dt, Value::string("America/New_York")]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let hour_key = HashableValue::String(Rc::new("hour".to_string()));
            let tz_key = HashableValue::String(Rc::new("timezone".to_string()));

            // UTC 00:00 -> NYC -5 hours = 19:00 (previous day)
            assert_eq!(map.get(&hour_key), Some(&Value::Int(19)));
            assert_eq!(map.get(&tz_key), Some(&Value::string("America/New_York")));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_datetime_invalid_timezone() {
        let dt = datetime_method("now", &[]).unwrap();
        let result = datetime_method("to_timezone", &[dt, Value::string("Invalid/Timezone")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid timezone"));
    }

    #[test]
    fn test_datetime_wrong_args() {
        // Too many args
        let result = datetime_method("now", &[Value::Int(1)]);
        assert!(result.is_err());

        // Missing args
        let result = datetime_method("parse", &[]);
        assert!(result.is_err());
    }

    // ============================================================================
    // Duration Module Tests
    // ============================================================================

    #[test]
    fn test_duration_creation() {
        // Milliseconds
        let result = duration_method("milliseconds", &[Value::Int(500)]).unwrap();
        let millis = duration_method("as_millis", &[result]).unwrap();
        assert_eq!(millis, Value::Int(500));

        // Seconds
        let result = duration_method("seconds", &[Value::Int(2)]).unwrap();
        let millis = duration_method("as_millis", &[result]).unwrap();
        assert_eq!(millis, Value::Int(2000));

        // Minutes
        let result = duration_method("minutes", &[Value::Int(5)]).unwrap();
        let millis = duration_method("as_millis", &[result]).unwrap();
        assert_eq!(millis, Value::Int(300_000));

        // Hours
        let result = duration_method("hours", &[Value::Int(1)]).unwrap();
        let millis = duration_method("as_millis", &[result]).unwrap();
        assert_eq!(millis, Value::Int(3_600_000));

        // Days
        let result = duration_method("days", &[Value::Int(1)]).unwrap();
        let millis = duration_method("as_millis", &[result]).unwrap();
        assert_eq!(millis, Value::Int(86_400_000));
    }

    #[test]
    fn test_duration_conversion() {
        let one_hour = duration_method("hours", &[Value::Int(1)]).unwrap();

        // as_secs
        let secs = duration_method("as_secs", &[one_hour.clone()]).unwrap();
        assert_eq!(secs, Value::Float(3600.0));

        // as_mins
        let mins = duration_method("as_mins", &[one_hour.clone()]).unwrap();
        assert_eq!(mins, Value::Float(60.0));

        // as_hours
        let hours = duration_method("as_hours", &[one_hour.clone()]).unwrap();
        assert_eq!(hours, Value::Float(1.0));

        // as_days
        let days = duration_method("as_days", &[one_hour]).unwrap();
        if let Value::Float(d) = days {
            assert!((d - (1.0 / 24.0)).abs() < 0.0001);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_duration_arithmetic() {
        let d1 = duration_method("hours", &[Value::Int(1)]).unwrap();
        let d2 = duration_method("minutes", &[Value::Int(30)]).unwrap();

        // Add
        let sum = duration_method("add", &[d1.clone(), d2.clone()]).unwrap();
        let millis = duration_method("as_millis", &[sum]).unwrap();
        assert_eq!(millis, Value::Int(5_400_000)); // 1.5 hours

        // Subtract
        let diff = duration_method("subtract", &[d1, d2]).unwrap();
        let millis = duration_method("as_millis", &[diff]).unwrap();
        assert_eq!(millis, Value::Int(1_800_000)); // 30 minutes
    }

    #[test]
    fn test_duration_wrong_args() {
        // Missing args
        let result = duration_method("seconds", &[]);
        assert!(result.is_err());

        // Wrong type
        let result = duration_method("seconds", &[Value::string("foo")]);
        assert!(result.is_err());
    }

    // ============================================================================
    // Time Module Tests
    // ============================================================================

    #[test]
    fn test_time_start_elapsed() {
        let timer = time_method("start", &[]).unwrap();

        // Small sleep
        std::thread::sleep(std::time::Duration::from_millis(10));

        let elapsed = time_method("elapsed", &[timer]).unwrap();
        let millis = duration_method("as_millis", &[elapsed]).unwrap();

        if let Value::Int(ms) = millis {
            assert!(ms >= 10, "Elapsed time should be at least 10ms, got {}", ms);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_time_sleep_ms() {
        let start = std::time::Instant::now();
        time_method("sleep_ms", &[Value::Int(10)]).unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() >= 10, "Should have slept at least 10ms");
    }

    #[test]
    fn test_time_sleep_with_duration() {
        let duration = duration_method("milliseconds", &[Value::Int(10)]).unwrap();
        let start = std::time::Instant::now();
        time_method("sleep", &[duration]).unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() >= 10, "Should have slept at least 10ms");
    }

    #[test]
    fn test_time_sleep_negative() {
        let result = time_method("sleep_ms", &[Value::Int(-100)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be negative"));
    }

    #[test]
    fn test_time_wrong_args() {
        // start with args
        let result = time_method("start", &[Value::Int(1)]);
        assert!(result.is_err());

        // elapsed without timer
        let result = time_method("elapsed", &[Value::Int(1)]);
        assert!(result.is_err());
    }

    // ============================================================================
    // Dispatch Tests for DateTime/Duration/Time
    // ============================================================================

    #[test]
    fn test_dispatch_datetime_namespace() {
        let result = dispatch_namespace_method("DateTime", "now", &[]).unwrap();
        assert!(matches!(result, Value::Map(_)));
    }

    #[test]
    fn test_dispatch_duration_namespace() {
        let result = dispatch_namespace_method("Duration", "seconds", &[Value::Int(5)]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new("millis".to_string()));
            assert_eq!(map.get(&key), Some(&Value::Int(5000)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_dispatch_time_namespace() {
        let result = dispatch_namespace_method("Time", "start", &[]).unwrap();
        assert!(matches!(result, Value::Map(_)));
    }

    // ============================================================================
    // Regex Module Tests
    // ============================================================================

    #[test]
    fn test_regex_new() {
        // Create a compiled regex
        let result = regex_method("new", &[Value::string(r"\d+")]);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::Regex(_)));
    }

    #[test]
    fn test_regex_new_with_options() {
        // Create a case-insensitive regex
        let mut options = HashMap::new();
        options.insert(
            HashableValue::String(Rc::new("case_insensitive".to_string())),
            Value::Bool(true),
        );
        let result = regex_method(
            "new",
            &[
                Value::string("hello"),
                Value::Map(Rc::new(RefCell::new(options))),
            ],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_regex_new_invalid_pattern() {
        // Invalid regex pattern should error
        let result = regex_method("new", &[Value::string(r"[invalid")]);
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_is_match_with_pattern() {
        // Using pattern string
        let result = regex_method(
            "is_match",
            &[Value::string(r"\d+"), Value::string("hello 123 world")],
        );
        assert_eq!(result, Ok(Value::Bool(true)));

        let result = regex_method(
            "is_match",
            &[Value::string(r"\d+"), Value::string("hello world")],
        );
        assert_eq!(result, Ok(Value::Bool(false)));
    }

    #[test]
    fn test_regex_is_match_with_compiled() {
        // Using compiled regex
        let re = regex_method("new", &[Value::string(r"\d+")]).unwrap();
        let result = regex_method("is_match", &[re, Value::string("abc 123 def")]);
        assert_eq!(result, Ok(Value::Bool(true)));
    }

    #[test]
    fn test_regex_find() {
        let result = regex_method(
            "find",
            &[Value::string(r"\d+"), Value::string("hello 123 world")],
        )
        .unwrap();

        if let Value::Map(map) = result {
            let map = map.borrow();
            let text_key = HashableValue::String(Rc::new("text".to_string()));
            let start_key = HashableValue::String(Rc::new("start".to_string()));
            let end_key = HashableValue::String(Rc::new("end".to_string()));

            assert_eq!(map.get(&text_key), Some(&Value::string("123")));
            assert_eq!(map.get(&start_key), Some(&Value::Int(6)));
            assert_eq!(map.get(&end_key), Some(&Value::Int(9)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_regex_find_no_match() {
        let result = regex_method(
            "find",
            &[Value::string(r"\d+"), Value::string("hello world")],
        )
        .unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_regex_find_all() {
        let result = regex_method(
            "find_all",
            &[Value::string(r"\d+"), Value::string("a1b2c3")],
        )
        .unwrap();

        if let Value::List(list) = result {
            let list = list.borrow();
            assert_eq!(list.len(), 3);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_regex_replace() {
        let result = regex_method(
            "replace",
            &[
                Value::string(r"\d+"),
                Value::string("hello 123 world 456"),
                Value::string("X"),
            ],
        )
        .unwrap();
        assert_eq!(result, Value::string("hello X world 456"));
    }

    #[test]
    fn test_regex_replace_all() {
        let result = regex_method(
            "replace_all",
            &[
                Value::string(r"\d+"),
                Value::string("hello 123 world 456"),
                Value::string("X"),
            ],
        )
        .unwrap();
        assert_eq!(result, Value::string("hello X world X"));
    }

    #[test]
    fn test_regex_replace_with_capture_groups() {
        // Swap first and last name
        let result = regex_method(
            "replace",
            &[
                Value::string(r"(\w+), (\w+)"),
                Value::string("Doe, John"),
                Value::string("$2 $1"),
            ],
        )
        .unwrap();
        assert_eq!(result, Value::string("John Doe"));
    }

    #[test]
    fn test_regex_split() {
        let result = regex_method(
            "split",
            &[Value::string(r"\s+"), Value::string("hello   world   foo")],
        )
        .unwrap();

        if let Value::List(list) = result {
            let list = list.borrow();
            assert_eq!(list.len(), 3);
            assert_eq!(list[0], Value::string("hello"));
            assert_eq!(list[1], Value::string("world"));
            assert_eq!(list[2], Value::string("foo"));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_regex_captures() {
        let result = regex_method(
            "captures",
            &[
                Value::string(r"(\w+)@(\w+)\.(\w+)"),
                Value::string("user@example.com"),
            ],
        )
        .unwrap();

        if let Value::List(list) = result {
            let list = list.borrow();
            assert_eq!(list.len(), 4); // full match + 3 groups
            assert_eq!(list[0], Value::string("user@example.com")); // full match
            assert_eq!(list[1], Value::string("user"));
            assert_eq!(list[2], Value::string("example"));
            assert_eq!(list[3], Value::string("com"));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_regex_captures_no_match() {
        let result = regex_method(
            "captures",
            &[Value::string(r"(\d+)"), Value::string("hello")],
        )
        .unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_regex_case_insensitive() {
        let mut options = HashMap::new();
        options.insert(
            HashableValue::String(Rc::new("case_insensitive".to_string())),
            Value::Bool(true),
        );
        let opts = Value::Map(Rc::new(RefCell::new(options)));

        let result = regex_method(
            "is_match",
            &[Value::string("hello"), opts, Value::string("HELLO WORLD")],
        );
        assert_eq!(result, Ok(Value::Bool(true)));
    }

    #[test]
    fn test_regex_multiline() {
        let mut options = HashMap::new();
        options.insert(
            HashableValue::String(Rc::new("multiline".to_string())),
            Value::Bool(true),
        );
        let opts = Value::Map(Rc::new(RefCell::new(options)));

        // ^ should match start of each line in multiline mode
        let result = regex_method(
            "find_all",
            &[Value::string("^hello"), opts, Value::string("hello\nhello")],
        )
        .unwrap();

        if let Value::List(list) = result {
            assert_eq!(list.borrow().len(), 2);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_dispatch_regex_namespace() {
        let result = dispatch_namespace_method(
            "Regex",
            "is_match",
            &[Value::string(r"\d+"), Value::string("test 123")],
        );
        assert_eq!(result, Ok(Value::Bool(true)));
    }

    // ============================================================================
    // Hash Module Tests
    // ============================================================================

    #[test]
    fn test_hash_sha256() {
        let result = hash_method("sha256", &[Value::string("hello")]).unwrap();
        // Known SHA-256 hash of "hello"
        assert_eq!(
            result,
            Value::string("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
        );
    }

    #[test]
    fn test_hash_sha512() {
        let result = hash_method("sha512", &[Value::string("hello")]).unwrap();
        // SHA-512 produces 128 hex chars
        if let Value::String(s) = result {
            assert_eq!(s.len(), 128);
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_hash_md5() {
        let result = hash_method("md5", &[Value::string("hello")]).unwrap();
        // Known MD5 hash of "hello"
        assert_eq!(result, Value::string("5d41402abc4b2a76b9719d911017c592"));
    }

    #[test]
    fn test_hash_hmac_sha256() {
        let result = hash_method(
            "hmac_sha256",
            &[Value::string("key"), Value::string("message")],
        )
        .unwrap();
        // HMAC-SHA256 produces 64 hex chars
        if let Value::String(s) = result {
            assert_eq!(s.len(), 64);
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_dispatch_hash_namespace() {
        let result = dispatch_namespace_method("Hash", "sha256", &[Value::string("test")]);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Uuid Module Tests
    // ============================================================================

    #[test]
    fn test_uuid_v4() {
        let result = uuid_method("v4", &[]).unwrap();
        if let Value::String(s) = result {
            // UUIDv4 format: 8-4-4-4-12
            assert_eq!(s.len(), 36);
            assert!(s.contains('-'));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_uuid_v7() {
        let result = uuid_method("v7", &[]).unwrap();
        if let Value::String(s) = result {
            assert_eq!(s.len(), 36);
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_uuid_parse() {
        let result = uuid_method(
            "parse",
            &[Value::string("550e8400-e29b-41d4-a716-446655440000")],
        )
        .unwrap();
        assert_eq!(
            result,
            Value::string("550e8400-e29b-41d4-a716-446655440000")
        );
    }

    #[test]
    fn test_uuid_parse_uppercase() {
        // Should normalize to lowercase
        let result = uuid_method(
            "parse",
            &[Value::string("550E8400-E29B-41D4-A716-446655440000")],
        )
        .unwrap();
        assert_eq!(
            result,
            Value::string("550e8400-e29b-41d4-a716-446655440000")
        );
    }

    #[test]
    fn test_uuid_is_valid() {
        assert_eq!(
            uuid_method(
                "is_valid",
                &[Value::string("550e8400-e29b-41d4-a716-446655440000")]
            ),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            uuid_method("is_valid", &[Value::string("not-a-uuid")]),
            Ok(Value::Bool(false))
        );
    }

    #[test]
    fn test_dispatch_uuid_namespace() {
        let result = dispatch_namespace_method("Uuid", "v4", &[]);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Random Module Tests
    // ============================================================================

    #[test]
    fn test_random_int() {
        let result = random_method("int", &[Value::Int(1), Value::Int(10)]).unwrap();
        if let Value::Int(n) = result {
            assert!(n >= 1 && n <= 10);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_random_int_same_bounds() {
        let result = random_method("int", &[Value::Int(5), Value::Int(5)]).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_random_int_invalid_range() {
        let result = random_method("int", &[Value::Int(10), Value::Int(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_random_float() {
        let result = random_method("float", &[]).unwrap();
        if let Value::Float(f) = result {
            assert!(f >= 0.0 && f < 1.0);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_random_bool() {
        let result = random_method("bool", &[]).unwrap();
        assert!(matches!(result, Value::Bool(_)));
    }

    #[test]
    fn test_random_choice() {
        let list = Value::List(Rc::new(RefCell::new(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
        ])));
        let result = random_method("choice", &[list]).unwrap();
        if let Value::Int(n) = result {
            assert!(n >= 1 && n <= 3);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_random_choice_empty() {
        let list = Value::List(Rc::new(RefCell::new(vec![])));
        let result = random_method("choice", &[list]);
        assert!(result.is_err());
    }

    #[test]
    fn test_random_shuffle() {
        let list = Value::List(Rc::new(RefCell::new(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
        ])));
        let result = random_method("shuffle", &[list]).unwrap();
        if let Value::List(shuffled) = result {
            // Same length
            assert_eq!(shuffled.borrow().len(), 3);
            // All elements present (sum should be same)
            let sum: i64 = shuffled
                .borrow()
                .iter()
                .map(|v| if let Value::Int(n) = v { *n } else { 0 })
                .sum();
            assert_eq!(sum, 6);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_random_bytes() {
        let result = random_method("bytes", &[Value::Int(16)]).unwrap();
        if let Value::List(bytes) = result {
            assert_eq!(bytes.borrow().len(), 16);
            // All values should be 0-255
            for v in bytes.borrow().iter() {
                if let Value::Int(n) = v {
                    assert!(*n >= 0 && *n <= 255);
                } else {
                    panic!("Expected Int in bytes");
                }
            }
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_random_bytes_zero() {
        let result = random_method("bytes", &[Value::Int(0)]).unwrap();
        if let Value::List(bytes) = result {
            assert_eq!(bytes.borrow().len(), 0);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_dispatch_random_namespace() {
        let result = dispatch_namespace_method("Random", "float", &[]);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Math Module Tests
    // ============================================================================

    #[test]
    fn test_math_constants() {
        // PI
        let result = math_method("PI", &[]).unwrap();
        assert_eq!(result, Value::Float(std::f64::consts::PI));

        // E
        let result = math_method("E", &[]).unwrap();
        assert_eq!(result, Value::Float(std::f64::consts::E));

        // TAU
        let result = math_method("TAU", &[]).unwrap();
        assert_eq!(result, Value::Float(std::f64::consts::TAU));

        // INFINITY
        let result = math_method("INFINITY", &[]).unwrap();
        if let Value::Float(f) = result {
            assert!(f.is_infinite() && f.is_sign_positive());
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_math_abs() {
        // Int positive
        let result = math_method("abs", &[Value::Int(5)]).unwrap();
        assert_eq!(result, Value::Int(5));

        // Int negative
        let result = math_method("abs", &[Value::Int(-5)]).unwrap();
        assert_eq!(result, Value::Int(5));

        // Float
        let result = math_method("abs", &[Value::Float(-3.14)]).unwrap();
        assert_eq!(result, Value::Float(3.14));
    }

    #[test]
    fn test_math_floor_ceil_round() {
        // floor
        let result = math_method("floor", &[Value::Float(3.7)]).unwrap();
        assert_eq!(result, Value::Int(3));

        let result = math_method("floor", &[Value::Float(-3.7)]).unwrap();
        assert_eq!(result, Value::Int(-4));

        // ceil
        let result = math_method("ceil", &[Value::Float(3.2)]).unwrap();
        assert_eq!(result, Value::Int(4));

        // round
        let result = math_method("round", &[Value::Float(3.5)]).unwrap();
        assert_eq!(result, Value::Int(4));

        let result = math_method("round", &[Value::Float(3.4)]).unwrap();
        assert_eq!(result, Value::Int(3));
    }

    #[test]
    fn test_math_trunc_sign_fract() {
        // trunc
        let result = math_method("trunc", &[Value::Float(3.7)]).unwrap();
        assert_eq!(result, Value::Int(3));

        let result = math_method("trunc", &[Value::Float(-3.7)]).unwrap();
        assert_eq!(result, Value::Int(-3));

        // sign
        let result = math_method("sign", &[Value::Int(5)]).unwrap();
        assert_eq!(result, Value::Int(1));

        let result = math_method("sign", &[Value::Int(-5)]).unwrap();
        assert_eq!(result, Value::Int(-1));

        let result = math_method("sign", &[Value::Int(0)]).unwrap();
        assert_eq!(result, Value::Int(0));

        // fract
        let result = math_method("fract", &[Value::Float(3.75)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 0.75).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_math_trig() {
        // sin(0) = 0
        let result = math_method("sin", &[Value::Float(0.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!(f.abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // cos(0) = 1
        let result = math_method("cos", &[Value::Float(0.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 1.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // sin(PI/2) = 1
        let result = math_method("sin", &[Value::Float(std::f64::consts::FRAC_PI_2)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 1.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // tan(0) = 0
        let result = math_method("tan", &[Value::Float(0.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!(f.abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_math_inverse_trig() {
        // asin(0) = 0
        let result = math_method("asin", &[Value::Float(0.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!(f.abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // acos(1) = 0
        let result = math_method("acos", &[Value::Float(1.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!(f.abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // atan(0) = 0
        let result = math_method("atan", &[Value::Float(0.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!(f.abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // atan2(1, 1) = PI/4
        let result = math_method("atan2", &[Value::Float(1.0), Value::Float(1.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_math_hyperbolic() {
        // sinh(0) = 0
        let result = math_method("sinh", &[Value::Float(0.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!(f.abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // cosh(0) = 1
        let result = math_method("cosh", &[Value::Float(0.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 1.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // tanh(0) = 0
        let result = math_method("tanh", &[Value::Float(0.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!(f.abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_math_exp_log() {
        // exp(0) = 1
        let result = math_method("exp", &[Value::Float(0.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 1.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // exp(1) = e
        let result = math_method("exp", &[Value::Float(1.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - std::f64::consts::E).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // ln(1) = 0
        let result = math_method("ln", &[Value::Float(1.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!(f.abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // ln(e) = 1
        let result = math_method("ln", &[Value::Float(std::f64::consts::E)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 1.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // log2(8) = 3
        let result = math_method("log2", &[Value::Float(8.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 3.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // log10(100) = 2
        let result = math_method("log10", &[Value::Float(100.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 2.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // exp2(3) = 8
        let result = math_method("exp2", &[Value::Float(3.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 8.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_math_pow_sqrt_cbrt() {
        // pow(2, 3) = 8
        let result = math_method("pow", &[Value::Float(2.0), Value::Float(3.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 8.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // sqrt(16) = 4
        let result = math_method("sqrt", &[Value::Float(16.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 4.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // cbrt(27) = 3
        let result = math_method("cbrt", &[Value::Float(27.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 3.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_math_min_max() {
        // min with ints
        let result = math_method("min", &[Value::Int(3), Value::Int(1), Value::Int(2)]).unwrap();
        assert_eq!(result, Value::Int(1));

        // max with ints
        let result = math_method("max", &[Value::Int(3), Value::Int(1), Value::Int(2)]).unwrap();
        assert_eq!(result, Value::Int(3));

        // min with floats
        let result = math_method("min", &[Value::Float(3.5), Value::Float(1.5)]).unwrap();
        assert_eq!(result, Value::Float(1.5));

        // max with single value
        let result = math_method("max", &[Value::Int(42)]).unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_math_clamp() {
        // value within range
        let result = math_method("clamp", &[Value::Int(5), Value::Int(0), Value::Int(10)]).unwrap();
        assert_eq!(result, Value::Int(5));

        // value below min
        let result =
            math_method("clamp", &[Value::Int(-5), Value::Int(0), Value::Int(10)]).unwrap();
        assert_eq!(result, Value::Int(0));

        // value above max
        let result =
            math_method("clamp", &[Value::Int(15), Value::Int(0), Value::Int(10)]).unwrap();
        assert_eq!(result, Value::Int(10));

        // invalid range (min > max)
        let result = math_method("clamp", &[Value::Int(5), Value::Int(10), Value::Int(0)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_math_hypot() {
        // 3, 4, 5 triangle
        let result = math_method("hypot", &[Value::Float(3.0), Value::Float(4.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 5.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_math_degrees_radians() {
        // 180 degrees = PI radians
        let result = math_method("to_radians", &[Value::Float(180.0)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - std::f64::consts::PI).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }

        // PI radians = 180 degrees
        let result = math_method("to_degrees", &[Value::Float(std::f64::consts::PI)]).unwrap();
        if let Value::Float(f) = result {
            assert!((f - 180.0).abs() < 1e-10);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_math_is_nan_infinite_finite() {
        // is_nan
        assert_eq!(
            math_method("is_nan", &[Value::Float(f64::NAN)]),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            math_method("is_nan", &[Value::Float(1.0)]),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            math_method("is_nan", &[Value::Int(1)]),
            Ok(Value::Bool(false))
        );

        // is_infinite
        assert_eq!(
            math_method("is_infinite", &[Value::Float(f64::INFINITY)]),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            math_method("is_infinite", &[Value::Float(f64::NEG_INFINITY)]),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            math_method("is_infinite", &[Value::Float(1.0)]),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            math_method("is_infinite", &[Value::Int(1)]),
            Ok(Value::Bool(false))
        );

        // is_finite
        assert_eq!(
            math_method("is_finite", &[Value::Float(1.0)]),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            math_method("is_finite", &[Value::Float(f64::INFINITY)]),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            math_method("is_finite", &[Value::Int(1)]),
            Ok(Value::Bool(true))
        );
    }

    #[test]
    fn test_math_wrong_args() {
        // abs with wrong type
        let result = math_method("abs", &[Value::string("hello")]);
        assert!(result.is_err());

        // abs with wrong arity
        let result = math_method("abs", &[]);
        assert!(result.is_err());

        // pow with wrong arity
        let result = math_method("pow", &[Value::Int(2)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_math_unknown_method() {
        let result = math_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    #[test]
    fn test_dispatch_math_namespace() {
        let result = dispatch_namespace_method("Math", "PI", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Float(std::f64::consts::PI));
    }

    // ============================================================================
    // Input Module Tests
    // ============================================================================
    // Note: Most Input functions require actual stdin interaction, so we test
    // error handling and argument validation rather than full functionality.

    #[test]
    fn test_input_read_line_invalid_args() {
        // read_line() takes no arguments
        let result = input_method("read_line", &[Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 0 arguments"));
    }

    #[test]
    fn test_input_read_all_invalid_args() {
        // read_all() takes no arguments
        let result = input_method("read_all", &[Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 0 arguments"));
    }

    #[test]
    fn test_input_prompt_invalid_args() {
        // prompt() requires exactly 1 string argument
        let result = input_method("prompt", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));

        let result = input_method("prompt", &[Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be String"));
    }

    #[test]
    fn test_input_prompt_int_invalid_args() {
        // prompt_int() requires exactly 1 string argument
        let result = input_method("prompt_int", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));
    }

    #[test]
    fn test_input_prompt_bool_invalid_args() {
        // prompt_bool() requires exactly 1 string argument
        let result = input_method("prompt_bool", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));
    }

    #[test]
    fn test_input_prompt_secret_invalid_args() {
        // prompt_secret() requires exactly 1 string argument
        let result = input_method("prompt_secret", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));
    }

    #[test]
    fn test_input_choose_invalid_args() {
        // choose() requires exactly 2 arguments
        let result = input_method("choose", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));

        // Second argument must be a list
        let result = input_method("choose", &[Value::string("Pick one:"), Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be List"));

        // List cannot be empty
        let empty_list = Value::list(vec![]);
        let result = input_method("choose", &[Value::string("Pick one:"), empty_list]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));

        // List items must be strings
        let int_list = Value::list(vec![Value::Int(1), Value::Int(2)]);
        let result = input_method("choose", &[Value::string("Pick one:"), int_list]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be strings"));
    }

    #[test]
    fn test_input_unknown_method() {
        let result = input_method("unknown_method", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    #[test]
    fn test_dispatch_input_namespace() {
        // Verify Input is properly routed through dispatch
        let result = dispatch_namespace_method("Input", "read_line", &[Value::Int(1)]);
        assert!(result.is_err()); // Fails due to wrong arg count, but proves routing works
        assert!(result.unwrap_err().contains("expects 0 arguments"));
    }

    // ============================================================================
    // Log Module Tests
    // ============================================================================

    #[test]
    fn test_log_set_level() {
        // Test that all valid levels are accepted
        // Note: We can't reliably check the level value due to parallel tests
        // modifying shared global state
        let result = log_method("set_level", &[Value::string("debug")]);
        assert!(result.is_ok());

        let result = log_method("set_level", &[Value::string("info")]);
        assert!(result.is_ok());

        let result = log_method("set_level", &[Value::string("warn")]);
        assert!(result.is_ok());

        let result = log_method("set_level", &[Value::string("warning")]);
        assert!(result.is_ok());

        let result = log_method("set_level", &[Value::string("error")]);
        assert!(result.is_ok());

        // Invalid level should fail
        let result = log_method("set_level", &[Value::string("invalid")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid log level"));
    }

    #[test]
    fn test_log_level_returns_string() {
        // Just verify level() returns a valid level string
        let result = log_method("level", &[]).unwrap();
        if let Value::String(s) = result {
            let valid_levels = ["debug", "info", "warn", "error"];
            assert!(
                valid_levels.contains(&s.as_str()),
                "level() returned unexpected value: {}",
                s
            );
        } else {
            panic!("level() should return a String");
        }
    }

    #[test]
    fn test_log_set_level_arg_validation() {
        // Wrong number of arguments
        let result = log_method("set_level", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));

        // Wrong type
        let result = log_method("set_level", &[Value::Int(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_log_level_no_args() {
        let result = log_method("level", &[Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 0 arguments"));
    }

    #[test]
    fn test_log_to_file() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        let path_str = log_path.to_string_lossy().to_string();

        // Set output to file
        let result = log_method("to_file", &[Value::string(&path_str)]);
        assert!(result.is_ok());

        // Reset to stdout for other tests
        let _ = log_method("to_stdout", &[]);
    }

    #[test]
    fn test_log_to_stderr() {
        let result = log_method("to_stderr", &[]);
        assert!(result.is_ok());

        // Reset to stdout
        let _ = log_method("to_stdout", &[]);
    }

    #[test]
    fn test_log_to_stdout() {
        let result = log_method("to_stdout", &[]);
        assert!(result.is_ok());

        // Wrong args
        let result = log_method("to_stdout", &[Value::Int(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_log_set_format() {
        let result = log_method("set_format", &[Value::string("{message}")]);
        assert!(result.is_ok());

        // Reset to default format
        let _ = log_method(
            "set_format",
            &[Value::string("[{level}] {timestamp} - {message}")],
        );
    }

    #[test]
    fn test_log_set_format_arg_validation() {
        // Wrong number of arguments
        let result = log_method("set_format", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));

        // Wrong type
        let result = log_method("set_format", &[Value::Int(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_log_message_arg_validation() {
        // No arguments
        let result = log_method("info", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1-2 arguments"));

        // Too many arguments
        let result = log_method(
            "info",
            &[Value::string("a"), Value::string("b"), Value::string("c")],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1-2 arguments"));

        // Wrong type for context (should be Map)
        let result = log_method("info", &[Value::string("msg"), Value::string("not a map")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("context must be a Map"));
    }

    #[test]
    fn test_log_messages_with_levels() {
        // Set to debug level so all messages pass through
        let _ = log_method("set_level", &[Value::string("debug")]);
        let _ = log_method("to_stdout", &[]);

        // All levels should succeed with valid args
        let result = log_method("debug", &[Value::string("Debug message")]);
        assert!(result.is_ok());

        let result = log_method("info", &[Value::string("Info message")]);
        assert!(result.is_ok());

        let result = log_method("warn", &[Value::string("Warning message")]);
        assert!(result.is_ok());

        let result = log_method("warning", &[Value::string("Warning message via alias")]);
        assert!(result.is_ok());

        let result = log_method("error", &[Value::string("Error message")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_level_filtering() {
        // Set to error level - only error messages should be logged
        let _ = log_method("set_level", &[Value::string("error")]);
        let _ = log_method("to_stdout", &[]);

        // These should return Ok but not actually log (filtered by level)
        let result = log_method("debug", &[Value::string("Filtered debug")]);
        assert!(result.is_ok());

        let result = log_method("info", &[Value::string("Filtered info")]);
        assert!(result.is_ok());

        let result = log_method("warn", &[Value::string("Filtered warn")]);
        assert!(result.is_ok());

        // This should log
        let result = log_method("error", &[Value::string("Not filtered error")]);
        assert!(result.is_ok());

        // Reset level
        let _ = log_method("set_level", &[Value::string("info")]);
    }

    #[test]
    fn test_log_structured_logging() {
        let _ = log_method("set_level", &[Value::string("debug")]);
        let _ = log_method("to_stdout", &[]);

        // Log with context map
        let mut context = HashMap::new();
        context.insert(
            HashableValue::String(Rc::new("user_id".to_string())),
            Value::Int(123),
        );
        context.insert(
            HashableValue::String(Rc::new("action".to_string())),
            Value::string("login"),
        );

        let result = log_method(
            "info",
            &[
                Value::string("User logged in"),
                Value::Map(Rc::new(RefCell::new(context))),
            ],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_write_to_file_directly() {
        // Test the write_log_output function directly to avoid shared state issues
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("output.log");
        let path_str = log_path.to_string_lossy().to_string();

        // Write directly to file using the internal function
        let result = write_log_output(
            &LogOutput::File(path_str.clone()),
            "TEST: Direct log message",
        );
        assert!(result.is_ok());

        // Read the file and verify content
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("TEST: Direct log message"));
    }

    #[test]
    fn test_log_unknown_method() {
        let result = log_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method 'unknown'"));
    }

    #[test]
    fn test_dispatch_log_namespace() {
        // Verify Log is properly routed through dispatch
        let result = dispatch_namespace_method("Log", "level", &[]);
        assert!(result.is_ok());
    }

    // ============================================================================
    // System Module Tests
    // ============================================================================

    #[test]
    fn test_system_os() {
        let result = system_method("os", &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::String(s) => {
                // OS should be one of the known values
                let os = s.to_string();
                assert!(
                    ["macos", "linux", "windows", "freebsd", "netbsd", "openbsd"]
                        .contains(&os.as_str())
                );
            }
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_system_os_no_args() {
        let result = system_method("os", &[Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 0 arguments"));
    }

    #[test]
    fn test_system_arch() {
        let result = system_method("arch", &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::String(s) => {
                // Arch should be one of the known values
                let arch = s.to_string();
                assert!(["x86_64", "aarch64", "x86", "arm"].contains(&arch.as_str()));
            }
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_system_arch_no_args() {
        let result = system_method("arch", &[Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 0 arguments"));
    }

    #[test]
    fn test_system_cwd() {
        let result = system_method("cwd", &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::String(s) => {
                // CWD should be a non-empty path
                assert!(!s.is_empty());
            }
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_system_cwd_no_args() {
        let result = system_method("cwd", &[Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 0 arguments"));
    }

    #[test]
    fn test_system_set_cwd() {
        // Get current directory to restore later
        let original_cwd = std::env::current_dir().unwrap();
        let temp = tempdir().unwrap();
        let temp_path = temp.path().to_string_lossy().to_string();

        // Set to temp directory
        let result = system_method("set_cwd", &[Value::string(&temp_path)]);
        assert!(result.is_ok());

        // Verify it changed
        let new_cwd = std::env::current_dir().unwrap();
        assert_eq!(
            new_cwd.canonicalize().unwrap(),
            temp.path().canonicalize().unwrap()
        );

        // Restore original cwd
        std::env::set_current_dir(original_cwd).unwrap();
    }

    #[test]
    fn test_system_set_cwd_invalid_path() {
        let result = system_method("set_cwd", &[Value::string("/nonexistent/path/12345")]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("failed to set current directory"));
    }

    #[test]
    fn test_system_temp_dir() {
        let result = system_method("temp_dir", &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::String(s) => {
                // Temp dir should be a non-empty path
                assert!(!s.is_empty());
            }
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_system_temp_file() {
        let result = system_method("temp_file", &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::String(s) => {
                // Should be a non-empty path
                assert!(!s.is_empty());
                // File should exist
                assert!(Path::new(&*s).exists());
                // Clean up
                let _ = fs::remove_file(&*s);
            }
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_system_cpu_count() {
        let result = system_method("cpu_count", &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Int(n) => {
                // CPU count should be at least 1
                assert!(n >= 1);
            }
            _ => panic!("Expected Int"),
        }
    }

    #[test]
    fn test_system_total_memory() {
        let result = system_method("total_memory", &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Int(n) => {
                // Total memory should be at least some reasonable amount (1 MB)
                assert!(n >= 1_000_000);
            }
            _ => panic!("Expected Int"),
        }
    }

    #[test]
    fn test_system_unknown_method() {
        let result = system_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method 'unknown'"));
    }

    #[test]
    fn test_dispatch_system_namespace() {
        // Verify System is properly routed through dispatch
        let result = dispatch_namespace_method("System", "os", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_system_hostname() {
        let result = system_method("hostname", &[]);
        assert!(result.is_ok());
        // Hostname should be a string or null
        match result.unwrap() {
            Value::String(_) | Value::Null => {}
            _ => panic!("Expected String or Null"),
        }
    }

    #[test]
    fn test_system_hostname_no_args() {
        let result = system_method("hostname", &[Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 0 arguments"));
    }

    #[test]
    fn test_system_uptime() {
        let result = system_method("uptime", &[]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Int(seconds) => {
                assert!(seconds >= 0, "Uptime should be non-negative");
            }
            _ => panic!("Expected Int"),
        }
    }

    #[test]
    fn test_system_uptime_no_args() {
        let result = system_method("uptime", &[Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 0 arguments"));
    }

    // ============================================================================
    // Process Module Tests
    // ============================================================================

    #[test]
    fn test_process_spawn() {
        // Spawn a simple command that exits immediately
        #[cfg(unix)]
        let result = process_method("spawn", &[Value::string("true")]);
        #[cfg(windows)]
        let result = process_method(
            "spawn",
            &[
                Value::string("cmd"),
                Value::list(vec![Value::string("/C"), Value::string("echo hello")]),
            ],
        );

        assert!(result.is_ok());
        let map = result.unwrap();
        if let Value::Map(m) = map {
            let m = m.borrow();
            // Should have pid
            let pid_key = HashableValue::String(Rc::new("pid".to_string()));
            assert!(m.contains_key(&pid_key));
            if let Some(Value::Int(pid)) = m.get(&pid_key) {
                assert!(*pid > 0);
            } else {
                panic!("pid should be an Int");
            }
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_process_spawn_with_args() {
        #[cfg(unix)]
        let result = process_method(
            "spawn",
            &[
                Value::string("echo"),
                Value::list(vec![Value::string("hello")]),
            ],
        );
        #[cfg(windows)]
        let result = process_method(
            "spawn",
            &[
                Value::string("cmd"),
                Value::list(vec![Value::string("/C"), Value::string("echo hello")]),
            ],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_process_spawn_invalid_args() {
        // No args
        let result = process_method("spawn", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1-2 arguments"));

        // Wrong second arg type
        let result = process_method("spawn", &[Value::string("echo"), Value::Int(42)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects List"));
    }

    #[test]
    fn test_process_kill_invalid_args() {
        // No args
        let result = process_method("kill", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));

        // Negative pid
        let result = process_method("kill", &[Value::Int(-1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be non-negative"));
    }

    #[test]
    fn test_process_unknown_method() {
        let result = process_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method 'unknown'"));
    }

    #[test]
    fn test_dispatch_process_namespace() {
        // Verify Process is properly routed through dispatch
        let result = dispatch_namespace_method("Process", "spawn", &[]);
        assert!(result.is_err()); // Should fail due to missing args
        assert!(result.unwrap_err().contains("expects 1-2 arguments"));
    }

    // ============================================================================
    // Signal Module Tests
    // ============================================================================

    #[test]
    fn test_signal_handle_validates_signal() {
        // Valid signal with dummy closure (will fail on closure validation)
        let result = signal_method("handle", &[Value::string("SIGINT"), Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be a closure"));
    }

    #[test]
    fn test_signal_handle_invalid_signal() {
        let result = signal_method("handle", &[Value::string("INVALID"), Value::Int(1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid signal"));
    }

    #[test]
    fn test_signal_handle_invalid_args() {
        // No args
        let result = signal_method("handle", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));

        // Only one arg
        let result = signal_method("handle", &[Value::string("SIGINT")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));
    }

    #[test]
    fn test_signal_unknown_method() {
        let result = signal_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method 'unknown'"));
    }

    #[test]
    fn test_dispatch_signal_namespace() {
        // Verify Signal is properly routed through dispatch
        let result = dispatch_namespace_method("Signal", "handle", &[]);
        assert!(result.is_err()); // Should fail due to missing args
        assert!(result.unwrap_err().contains("expects 2 arguments"));
    }

    // ============================================================================
    // Database Module Tests (SQLite and DuckDB - no external server required)
    // ============================================================================

    #[test]
    fn test_db_sqlite_memory() {
        // Create in-memory SQLite database
        let result = db_method("sqlite", &[Value::string(":memory:")]);
        assert!(result.is_ok());
        let conn = result.unwrap();
        assert!(matches!(conn, Value::DbConnection(_)));
    }

    #[test]
    fn test_db_sqlite_create_and_query() {
        // Create in-memory database
        let conn = db_method("sqlite", &[Value::string(":memory:")]).unwrap();
        let conn = match conn {
            Value::DbConnection(c) => c,
            _ => panic!("Expected DbConnection"),
        };

        // Create table
        let result = db_connection_method(
            &conn,
            "execute",
            &[Value::string(
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)",
            )],
        );
        assert!(result.is_ok());

        // Insert data
        let result = db_connection_method(
            &conn,
            "execute",
            &[
                Value::string("INSERT INTO users (name, age) VALUES (?, ?)"),
                Value::list(vec![Value::string("Alice"), Value::Int(30)]),
            ],
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(1)); // 1 row affected

        // Query data
        let result = db_connection_method(
            &conn,
            "query",
            &[
                Value::string("SELECT * FROM users WHERE name = ?"),
                Value::list(vec![Value::string("Alice")]),
            ],
        );
        assert!(result.is_ok());
        if let Value::List(rows) = result.unwrap() {
            let rows = rows.borrow();
            assert_eq!(rows.len(), 1);
            if let Value::Map(row) = &rows[0] {
                let row = row.borrow();
                let name_key = HashableValue::String(Rc::new("name".to_string()));
                assert_eq!(row.get(&name_key), Some(&Value::string("Alice")));
            }
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_db_sqlite_transaction() {
        let conn = db_method("sqlite", &[Value::string(":memory:")]).unwrap();
        let conn = match conn {
            Value::DbConnection(c) => c,
            _ => panic!("Expected DbConnection"),
        };

        // Create table
        db_connection_method(
            &conn,
            "execute",
            &[Value::string("CREATE TABLE test (id INTEGER)")],
        )
        .unwrap();

        // Begin transaction
        let result = db_connection_method(&conn, "begin", &[]);
        assert!(result.is_ok());

        // Insert data
        db_connection_method(
            &conn,
            "execute",
            &[Value::string("INSERT INTO test VALUES (1)")],
        )
        .unwrap();

        // Rollback
        let result = db_connection_method(&conn, "rollback", &[]);
        assert!(result.is_ok());

        // Verify data was rolled back
        let result = db_connection_method(
            &conn,
            "query",
            &[Value::string("SELECT COUNT(*) as count FROM test")],
        )
        .unwrap();
        if let Value::List(rows) = result {
            let rows = rows.borrow();
            if let Value::Map(row) = &rows[0] {
                let row = row.borrow();
                let count_key = HashableValue::String(Rc::new("count".to_string()));
                assert_eq!(row.get(&count_key), Some(&Value::Int(0)));
            }
        }
    }

    #[test]
    fn test_db_sqlite_metadata() {
        let conn = db_method("sqlite", &[Value::string(":memory:")]).unwrap();
        let conn = match conn {
            Value::DbConnection(c) => c,
            _ => panic!("Expected DbConnection"),
        };

        // Create table
        db_connection_method(
            &conn,
            "execute",
            &[Value::string(
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            )],
        )
        .unwrap();

        // List tables
        let tables = db_connection_method(&conn, "tables", &[]).unwrap();
        if let Value::List(tables) = tables {
            let tables = tables.borrow();
            assert_eq!(tables.len(), 1);
            assert_eq!(tables[0], Value::string("users"));
        }

        // Check table exists
        let exists =
            db_connection_method(&conn, "table_exists", &[Value::string("users")]).unwrap();
        assert_eq!(exists, Value::Bool(true));

        let exists =
            db_connection_method(&conn, "table_exists", &[Value::string("nonexistent")]).unwrap();
        assert_eq!(exists, Value::Bool(false));

        // Get columns
        let columns = db_connection_method(&conn, "columns", &[Value::string("users")]).unwrap();
        if let Value::List(columns) = columns {
            let columns = columns.borrow();
            assert_eq!(columns.len(), 2);
        }
    }

    #[test]
    fn test_db_sqlite_version() {
        let conn = db_method("sqlite", &[Value::string(":memory:")]).unwrap();
        let conn = match conn {
            Value::DbConnection(c) => c,
            _ => panic!("Expected DbConnection"),
        };

        let version = db_connection_method(&conn, "version", &[]).unwrap();
        if let Value::String(v) = version {
            assert!(v.starts_with("SQLite"));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_db_duckdb_memory() {
        // Create in-memory DuckDB database
        let result = db_method("duckdb", &[Value::string(":memory:")]);
        assert!(result.is_ok());
        let conn = result.unwrap();
        assert!(matches!(conn, Value::DbConnection(_)));
    }

    #[test]
    fn test_db_duckdb_create_and_query() {
        let conn = db_method("duckdb", &[Value::string(":memory:")]).unwrap();
        let conn = match conn {
            Value::DbConnection(c) => c,
            _ => panic!("Expected DbConnection"),
        };

        // Create table and insert data using execute
        db_connection_method(
            &conn,
            "execute",
            &[Value::string(
                "CREATE TABLE products (id INTEGER, name VARCHAR, price DOUBLE)",
            )],
        )
        .unwrap();

        db_connection_method(
            &conn,
            "execute",
            &[Value::string(
                "INSERT INTO products VALUES (1, 'Widget', 9.99)",
            )],
        )
        .unwrap();

        // Count query (simpler than SELECT *)
        let result = db_connection_method(
            &conn,
            "query",
            &[Value::string("SELECT COUNT(*) as cnt FROM products")],
        )
        .unwrap();
        if let Value::List(rows) = result {
            let rows = rows.borrow();
            assert_eq!(rows.len(), 1);
            // The row should contain the count
        }

        // Check version to verify connection works
        let version = db_connection_method(&conn, "version", &[]).unwrap();
        if let Value::String(v) = version {
            assert!(v.contains("DuckDB"));
        }
    }

    #[test]
    fn test_db_sqlite_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let path_str = db_path.to_string_lossy().to_string();

        // Create database file
        let conn = db_method("sqlite", &[Value::string(&path_str)]).unwrap();
        let conn = match conn {
            Value::DbConnection(c) => c,
            _ => panic!("Expected DbConnection"),
        };

        // Create table and insert data
        db_connection_method(
            &conn,
            "execute",
            &[Value::string("CREATE TABLE test (value TEXT)")],
        )
        .unwrap();
        db_connection_method(
            &conn,
            "execute",
            &[Value::string("INSERT INTO test VALUES ('hello')")],
        )
        .unwrap();

        // Close and reopen
        drop(conn);

        let conn = db_method("sqlite", &[Value::string(&path_str)]).unwrap();
        let conn = match conn {
            Value::DbConnection(c) => c,
            _ => panic!("Expected DbConnection"),
        };

        // Verify data persisted
        let result =
            db_connection_method(&conn, "query", &[Value::string("SELECT * FROM test")]).unwrap();
        if let Value::List(rows) = result {
            let rows = rows.borrow();
            assert_eq!(rows.len(), 1);
        }
    }

    #[test]
    fn test_db_unknown_method() {
        let result = db_method("unknown", &[Value::string("test")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    #[test]
    fn test_db_dispatch() {
        // Verify Db is properly routed through dispatch
        let result = dispatch_namespace_method("Db", "sqlite", &[Value::string(":memory:")]);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Async Module Tests
    // ============================================================================

    #[test]
    fn test_async_sleep() {
        let result = async_method("sleep", &[Value::Int(100)]);
        assert!(result.is_ok());
        let future = result.unwrap();
        match future {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("sleep"));
                assert_eq!(fut.metadata(), Some(&Value::Int(100)));
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_async_sleep_negative() {
        let result = async_method("sleep", &[Value::Int(-100)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be negative"));
    }

    #[test]
    fn test_async_sleep_no_args() {
        let result = async_method("sleep", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires a duration"));
    }

    #[test]
    fn test_async_sleep_wrong_type() {
        let result = async_method("sleep", &[Value::string("100")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires an Int"));
    }

    #[test]
    fn test_async_ready() {
        let result = async_method("ready", &[Value::Int(42)]);
        assert!(result.is_ok());
        let future = result.unwrap();
        match future {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_ready());
                assert_eq!(fut.result, Some(Value::Int(42)));
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_async_ready_no_args() {
        let result = async_method("ready", &[]);
        assert!(result.is_ok());
        let future = result.unwrap();
        match future {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_ready());
                assert_eq!(fut.result, Some(Value::Null));
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_async_failed() {
        let result = async_method("failed", &[Value::string("test error")]);
        assert!(result.is_ok());
        let future = result.unwrap();
        match future {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(!fut.is_pending());
                assert!(!fut.is_ready());
                match &fut.status {
                    crate::bytecode::FutureStatus::Failed(msg) => {
                        assert_eq!(msg, "test error");
                    }
                    _ => panic!("Expected Failed status"),
                }
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_async_failed_no_args() {
        let result = async_method("failed", &[]);
        assert!(result.is_ok());
        let future = result.unwrap();
        match future {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                match &fut.status {
                    crate::bytecode::FutureStatus::Failed(msg) => {
                        assert_eq!(msg, "unknown error");
                    }
                    _ => panic!("Expected Failed status"),
                }
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_async_unknown_method() {
        let result = async_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    #[test]
    fn test_dispatch_async_namespace() {
        let result = dispatch_namespace_method("Async", "ready", &[Value::Int(42)]);
        assert!(result.is_ok());
    }

    fn make_ready_future(value: Value) -> Value {
        let future = FutureState::ready(value);
        Value::Future(Rc::new(RefCell::new(future)))
    }

    fn make_pending_future_with_kind(kind: &str) -> Value {
        let future = FutureState::pending_with_metadata(Value::Null, kind.to_string());
        Value::Future(Rc::new(RefCell::new(future)))
    }

    #[test]
    fn test_async_all_with_list() {
        let futures = Value::list(vec![
            make_ready_future(Value::Int(1)),
            make_ready_future(Value::Int(2)),
            make_ready_future(Value::Int(3)),
        ]);

        let result = async_method("all", &[futures]).unwrap();
        match result {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("all"));
                assert!(matches!(fut.metadata(), Some(Value::List(_))));
            }
            _ => panic!("Expected Future"),
        }
    }

    #[test]
    fn test_async_all_empty_list() {
        let futures = Value::list(vec![]);
        let result = async_method("all", &[futures]).unwrap();
        match result {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("all"));
            }
            _ => panic!("Expected Future"),
        }
    }

    #[test]
    fn test_async_all_invalid_element() {
        let futures = Value::list(vec![
            make_ready_future(Value::Int(1)),
            Value::Int(42), // Not a future
        ]);

        let result = async_method("all", &[futures]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a Future"));
    }

    #[test]
    fn test_async_all_not_list() {
        let result = async_method("all", &[Value::Int(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_async_race_with_list() {
        let futures = Value::list(vec![
            make_pending_future_with_kind("sleep"),
            make_ready_future(Value::Int(42)),
        ]);

        let result = async_method("race", &[futures]).unwrap();
        match result {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("race"));
            }
            _ => panic!("Expected Future"),
        }
    }

    #[test]
    fn test_async_race_empty_list() {
        let futures = Value::list(vec![]);
        let result = async_method("race", &[futures]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one future"));
    }

    #[test]
    fn test_async_race_invalid_element() {
        let futures = Value::list(vec![Value::string("not a future")]);
        let result = async_method("race", &[futures]);
        assert!(result.is_err());
    }

    #[test]
    fn test_async_timeout() {
        let inner_future = make_pending_future_with_kind("sleep");
        let result = async_method("timeout", &[inner_future, Value::Int(1000)]).unwrap();

        match result {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("timeout"));
                if let Some(Value::Map(m)) = fut.metadata() {
                    let m = m.borrow();
                    assert!(m.contains_key(&HashableValue::String(Rc::new("future".into()))));
                    assert!(m.contains_key(&HashableValue::String(Rc::new("ms".into()))));
                } else {
                    panic!("Expected Map metadata");
                }
            }
            _ => panic!("Expected Future"),
        }
    }

    #[test]
    fn test_async_timeout_negative_ms() {
        let inner_future = make_pending_future_with_kind("sleep");
        let result = async_method("timeout", &[inner_future, Value::Int(-100)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_async_timeout_not_future() {
        let result = async_method("timeout", &[Value::Int(42), Value::Int(1000)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_async_spawn() {
        // Create a simple closure value for testing
        let func = crate::bytecode::Function {
            name: "test_closure".to_string(),
            arity: 0,
            upvalue_count: 0,
            chunk: crate::bytecode::Chunk::new(),
            execution_mode: crate::ast::ExecutionMode::Interpret,
        };
        let closure = crate::bytecode::Closure::new(Rc::new(func));
        let closure_val = Value::Closure(Rc::new(closure));

        let result = async_method("spawn", &[closure_val]).unwrap();
        match result {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("spawn"));
                assert!(matches!(fut.metadata(), Some(Value::Closure(_))));
            }
            _ => panic!("Expected Future"),
        }
    }

    #[test]
    fn test_async_spawn_not_closure() {
        let result = async_method("spawn", &[Value::Int(42)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("closure"));
    }

    // ============================================================================
    // TCP Module Tests
    // ============================================================================

    #[test]
    fn test_tcp_connect_creates_future() {
        let result = tcp_method("connect", &[Value::string("127.0.0.1"), Value::Int(8080)]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("tcp_connect"));
                assert_eq!(fut.metadata(), Some(&Value::string("127.0.0.1:8080")));
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_tcp_connect_validates_port() {
        // Port too high
        let result = tcp_method("connect", &[Value::string("localhost"), Value::Int(70000)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("port must be 1-65535"));

        // Port zero (invalid for connect)
        let result = tcp_method("connect", &[Value::string("localhost"), Value::Int(0)]);
        assert!(result.is_err());

        // Port negative
        let result = tcp_method("connect", &[Value::string("localhost"), Value::Int(-1)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_tcp_connect_wrong_args() {
        // Missing args
        let result = tcp_method("connect", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));

        // Wrong type for host
        let result = tcp_method("connect", &[Value::Int(123), Value::Int(8080)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("host must be String"));

        // Wrong type for port
        let result = tcp_method(
            "connect",
            &[Value::string("localhost"), Value::string("8080")],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("port must be Int"));
    }

    #[test]
    fn test_tcp_listen_creates_future() {
        let result = tcp_method("listen", &[Value::string("0.0.0.0"), Value::Int(0)]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("tcp_listen"));
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_tcp_unknown_method() {
        let result = tcp_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    #[test]
    fn test_dispatch_tcp_namespace() {
        let result = dispatch_namespace_method(
            "Tcp",
            "connect",
            &[Value::string("localhost"), Value::Int(80)],
        );
        assert!(result.is_ok());
    }

    // ============================================================================
    // UDP Module Tests
    // ============================================================================

    #[test]
    fn test_udp_bind_creates_future() {
        let result = udp_method("bind", &[Value::string("0.0.0.0"), Value::Int(0)]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("udp_bind"));
                assert_eq!(fut.metadata(), Some(&Value::string("0.0.0.0:0")));
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_udp_bind_validates_port() {
        // Port too high
        let result = udp_method("bind", &[Value::string("0.0.0.0"), Value::Int(70000)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("port must be 0-65535"));
    }

    #[test]
    fn test_udp_bind_wrong_args() {
        // Missing args
        let result = udp_method("bind", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));

        // Wrong type for addr
        let result = udp_method("bind", &[Value::Int(123), Value::Int(8080)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("addr must be String"));
    }

    #[test]
    fn test_udp_unknown_method() {
        let result = udp_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    #[test]
    fn test_dispatch_udp_namespace() {
        let result =
            dispatch_namespace_method("Udp", "bind", &[Value::string("0.0.0.0"), Value::Int(0)]);
        assert!(result.is_ok());
    }

    // ============================================================================
    // WebSocket Module Tests
    // ============================================================================

    #[test]
    fn test_ws_connect_creates_future() {
        let result = ws_method("connect", &[Value::string("ws://localhost:8080")]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("ws_connect"));
                assert_eq!(fut.metadata(), Some(&Value::string("ws://localhost:8080")));
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_ws_connect_validates_url_scheme() {
        // Missing ws:// or wss://
        let result = ws_method("connect", &[Value::string("http://localhost:8080")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must start with ws://"));

        // Valid wss:// scheme
        let result = ws_method("connect", &[Value::string("wss://localhost:8080/socket")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ws_connect_wrong_args() {
        // Missing args
        let result = ws_method("connect", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));

        // Wrong type for url
        let result = ws_method("connect", &[Value::Int(8080)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("url must be String"));
    }

    #[test]
    fn test_ws_listen_creates_future() {
        let result = ws_method("listen", &[Value::string("0.0.0.0"), Value::Int(0)]);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Future(fut_ref) => {
                let fut = fut_ref.borrow();
                assert!(fut.is_pending());
                assert_eq!(fut.kind(), Some("ws_listen"));
                assert_eq!(fut.metadata(), Some(&Value::string("0.0.0.0:0")));
            }
            _ => panic!("Expected Future value"),
        }
    }

    #[test]
    fn test_ws_listen_validates_port() {
        // Port too high
        let result = ws_method("listen", &[Value::string("localhost"), Value::Int(70000)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("port must be 0-65535"));

        // Port negative
        let result = ws_method("listen", &[Value::string("localhost"), Value::Int(-1)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_ws_listen_wrong_args() {
        // Missing args
        let result = ws_method("listen", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 2 arguments"));

        // Wrong type for addr
        let result = ws_method("listen", &[Value::Int(123), Value::Int(8080)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("addr must be String"));

        // Wrong type for port
        let result = ws_method(
            "listen",
            &[Value::string("localhost"), Value::string("8080")],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("port must be Int"));
    }

    #[test]
    fn test_ws_unknown_method() {
        let result = ws_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    #[test]
    fn test_dispatch_websocket_namespace() {
        let result = dispatch_namespace_method(
            "WebSocket",
            "connect",
            &[Value::string("ws://localhost:8080")],
        );
        assert!(result.is_ok());
    }

    // ============================================================================
    // Crypto Module Tests
    // ============================================================================

    #[test]
    fn test_crypto_random_bytes() {
        // Generate 32 random bytes
        let result = crypto_method("random_bytes", &[Value::Int(32)]).unwrap();
        if let Value::List(bytes) = result {
            assert_eq!(bytes.borrow().len(), 32);
            // All values should be in 0-255 range
            for b in bytes.borrow().iter() {
                if let Value::Int(i) = b {
                    assert!(*i >= 0 && *i <= 255);
                } else {
                    panic!("Expected Int in random bytes list");
                }
            }
        } else {
            panic!("Expected List from random_bytes");
        }
    }

    #[test]
    fn test_crypto_random_bytes_zero() {
        let result = crypto_method("random_bytes", &[Value::Int(0)]).unwrap();
        if let Value::List(bytes) = result {
            assert_eq!(bytes.borrow().len(), 0);
        } else {
            panic!("Expected empty List");
        }
    }

    #[test]
    fn test_crypto_random_bytes_error_negative() {
        let result = crypto_method("random_bytes", &[Value::Int(-1)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-negative"));
    }

    #[test]
    fn test_crypto_pbkdf2() {
        // Known test vector for PBKDF2-HMAC-SHA256
        let result = crypto_method(
            "pbkdf2",
            &[
                Value::string("password"),
                Value::string("salt"),
                Value::Int(1),
            ],
        )
        .unwrap();

        if let Value::String(key) = result {
            // Key should be 64 hex chars (32 bytes)
            assert_eq!(key.len(), 64);
            // Known value for 1 iteration
            assert_eq!(
                key.as_str(),
                "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
            );
        } else {
            panic!("Expected String from pbkdf2");
        }
    }

    #[test]
    fn test_crypto_pbkdf2_iterations() {
        // With more iterations, result should be different
        let result1 = crypto_method(
            "pbkdf2",
            &[
                Value::string("password"),
                Value::string("salt"),
                Value::Int(1000),
            ],
        )
        .unwrap();

        let result2 = crypto_method(
            "pbkdf2",
            &[
                Value::string("password"),
                Value::string("salt"),
                Value::Int(1),
            ],
        )
        .unwrap();

        assert_ne!(result1, result2);
    }

    #[test]
    fn test_crypto_pbkdf2_error_zero_iterations() {
        let result = crypto_method(
            "pbkdf2",
            &[
                Value::string("password"),
                Value::string("salt"),
                Value::Int(0),
            ],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 1"));
    }

    #[test]
    fn test_crypto_aes_roundtrip() {
        // Generate a key using pbkdf2
        let key = crypto_method(
            "pbkdf2",
            &[
                Value::string("test_password"),
                Value::string("test_salt"),
                Value::Int(1000),
            ],
        )
        .unwrap();

        let plaintext = "Hello, World! This is a test message.";

        // Encrypt
        let encrypted =
            crypto_method("aes_encrypt", &[Value::string(plaintext), key.clone()]).unwrap();

        // Encrypted should be base64 string
        if let Value::String(enc_str) = &encrypted {
            assert!(!enc_str.is_empty());
            // Should be valid base64
            assert!(base64::engine::general_purpose::STANDARD
                .decode(enc_str.as_str())
                .is_ok());
        } else {
            panic!("Expected String from aes_encrypt");
        }

        // Decrypt
        let decrypted = crypto_method("aes_decrypt", &[encrypted, key]).unwrap();

        assert_eq!(decrypted, Value::string(plaintext));
    }

    #[test]
    fn test_crypto_aes_different_key_fails() {
        // Encrypt with one key
        let key1 = crypto_method(
            "pbkdf2",
            &[
                Value::string("password1"),
                Value::string("salt"),
                Value::Int(1000),
            ],
        )
        .unwrap();

        let encrypted = crypto_method("aes_encrypt", &[Value::string("secret"), key1]).unwrap();

        // Try to decrypt with different key
        let key2 = crypto_method(
            "pbkdf2",
            &[
                Value::string("password2"),
                Value::string("salt"),
                Value::Int(1000),
            ],
        )
        .unwrap();

        let result = crypto_method("aes_decrypt", &[encrypted, key2]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Decryption failed"));
    }

    #[test]
    fn test_crypto_aes_invalid_key_length() {
        let result = crypto_method(
            "aes_encrypt",
            &[Value::string("test"), Value::string("short_key")],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("32 bytes"));
    }

    #[test]
    fn test_crypto_dispatch() {
        // Verify Crypto namespace is properly registered
        let result = dispatch_namespace_method("Crypto", "random_bytes", &[Value::Int(16)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_crypto_unknown_method() {
        let result = crypto_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    // ============================================================================
    // Test Module Tests
    // ============================================================================

    #[test]
    fn test_test_expect() {
        // Test.expect(value) should return an Expectation
        let result = test_method("expect", &[Value::Int(42)]);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(matches!(value, Value::Expectation(_)));
    }

    #[test]
    fn test_test_expect_wrong_args() {
        // Test.expect() with no args should fail
        let result = test_method("expect", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));
    }

    #[test]
    fn test_test_fail() {
        // Test.fail() should return an error
        let result = test_method("fail", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Test failed");
    }

    #[test]
    fn test_test_fail_with_message() {
        // Test.fail(message) should return the message as error
        let result = test_method("fail", &[Value::string("Custom error")]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Custom error");
    }

    #[test]
    fn test_test_skip() {
        // Test.skip() should return null
        let result = test_method("skip", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_test_pending() {
        // Test.pending() should return null
        let result = test_method("pending", &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_test_mock() {
        // Test.mock() should return a Map with mock structure
        let result = test_method("mock", &[]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            // Check __is_mock flag
            let key = HashableValue::String(Rc::new("__is_mock".to_string()));
            assert_eq!(map.get(&key), Some(&Value::Bool(true)));
            // Check call_count starts at 0
            let key = HashableValue::String(Rc::new("call_count".to_string()));
            assert_eq!(map.get(&key), Some(&Value::Int(0)));
        } else {
            panic!("Expected Map from mock()");
        }
    }

    #[test]
    fn test_test_mock_with_return_value() {
        // Test.mock(42) should set return_value to 42
        let result = test_method("mock", &[Value::Int(42)]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new("return_value".to_string()));
            assert_eq!(map.get(&key), Some(&Value::Int(42)));
        } else {
            panic!("Expected Map from mock()");
        }
    }

    #[test]
    fn test_test_spy() {
        // Test.spy() should return a Map with spy structure
        let result = test_method("spy", &[]).unwrap();
        if let Value::Map(map) = result {
            let map = map.borrow();
            let key = HashableValue::String(Rc::new("__is_spy".to_string()));
            assert_eq!(map.get(&key), Some(&Value::Bool(true)));
        } else {
            panic!("Expected Map from spy()");
        }
    }

    #[test]
    fn test_test_unknown_method() {
        let result = test_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method"));
    }

    #[test]
    fn test_dispatch_test_namespace() {
        // Verify Test namespace is properly routed through dispatch
        let result = dispatch_namespace_method("Test", "expect", &[Value::Int(1)]);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::Expectation(_)));
    }

    // ============================================================================
    // Math List Aggregate Functions Tests
    // ============================================================================

    fn make_number_list(nums: &[f64]) -> Value {
        use std::cell::RefCell;
        Value::List(Rc::new(RefCell::new(
            nums.iter().map(|&n| Value::Float(n)).collect(),
        )))
    }

    #[test]
    fn test_math_sum() {
        let list = make_number_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = math_method("sum", &[list]).unwrap();
        assert_eq!(result, Value::Float(15.0));
    }

    #[test]
    fn test_math_mean() {
        let list = make_number_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = math_method("mean", &[list]).unwrap();
        assert_eq!(result, Value::Float(3.0));
    }

    #[test]
    fn test_math_median_odd() {
        let list = make_number_list(&[1.0, 3.0, 2.0, 5.0, 4.0]);
        let result = math_method("median", &[list]).unwrap();
        assert_eq!(result, Value::Float(3.0));
    }

    #[test]
    fn test_math_median_even() {
        let list = make_number_list(&[1.0, 2.0, 3.0, 4.0]);
        let result = math_method("median", &[list]).unwrap();
        assert_eq!(result, Value::Float(2.5));
    }

    #[test]
    fn test_math_variance() {
        // Variance of [2, 4, 4, 4, 5, 5, 7, 9] = 4.0
        let list = make_number_list(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        let result = math_method("variance", &[list]).unwrap();
        assert_eq!(result, Value::Float(4.0));
    }

    #[test]
    fn test_math_std() {
        // Std of [2, 4, 4, 4, 5, 5, 7, 9] = 2.0
        let list = make_number_list(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        let result = math_method("std", &[list]).unwrap();
        assert_eq!(result, Value::Float(2.0));
    }

    #[test]
    fn test_math_round_to() {
        let result = math_method("round_to", &[Value::Float(3.14159), Value::Int(2)]).unwrap();
        assert_eq!(result, Value::Float(3.14));

        let result = math_method("round_to", &[Value::Float(3.14159), Value::Int(4)]).unwrap();
        assert_eq!(result, Value::Float(3.1416));
    }

    #[test]
    fn test_math_empty_list() {
        let empty = make_number_list(&[]);

        // All aggregate functions return NaN for empty lists
        if let Value::Float(f) = math_method("mean", &[empty.clone()]).unwrap() {
            assert!(f.is_nan());
        }
        if let Value::Float(f) = math_method("median", &[empty.clone()]).unwrap() {
            assert!(f.is_nan());
        }
    }

    // ============================================================================
    // XML Module Tests
    // ============================================================================

    #[test]
    fn test_xml_parse_simple() {
        let xml = "<root><item>hello</item></root>";
        let result = xml_method("parse", &[Value::string(xml)]);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::XmlDocument(_)));
    }

    #[test]
    fn test_xml_parse_invalid() {
        let result = xml_method("parse", &[Value::string("<unclosed>")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("XML parse error"));
    }

    #[test]
    fn test_xml_query_count() {
        let xml = "<root><item/><item/><item/></root>";
        let doc = xml_method("parse", &[Value::string(xml)]).unwrap();
        if let Value::XmlDocument(doc_ref) = doc {
            let result = xml_document_method(&doc_ref, "query", &[Value::string("count(//item)")]);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Value::Int(3));
        }
    }

    #[test]
    fn test_xml_root() {
        let xml = "<library><book/></library>";
        let doc = xml_method("parse", &[Value::string(xml)]).unwrap();
        if let Value::XmlDocument(doc_ref) = doc {
            let result = xml_document_method(&doc_ref, "root", &[]);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Value::string("library"));
        }
    }

    #[test]
    fn test_xml_stringify() {
        let xml = "<root><child>text</child></root>";
        let doc = xml_method("parse", &[Value::string(xml)]).unwrap();
        let result = xml_method("stringify", &[doc]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::string(xml));
    }

    #[test]
    fn test_xml_dispatch() {
        let result = dispatch_namespace_method("Xml", "parse", &[Value::string("<root/>")]);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::XmlDocument(_)));
    }

    // ============================================================================
    // Image Module Tests
    // ============================================================================

    #[test]
    fn test_image_new_basic() {
        let result = image_namespace_method("new", &[Value::Int(100), Value::Int(50)]);
        assert!(result.is_ok());
        if let Value::Image(img) = result.unwrap() {
            assert_eq!(img.width(), 100);
            assert_eq!(img.height(), 50);
        } else {
            panic!("Expected Image value");
        }
    }

    #[test]
    fn test_image_new_with_color() {
        let result = image_namespace_method(
            "new",
            &[Value::Int(10), Value::Int(10), Value::string("red")],
        );
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::Image(_)));
    }

    #[test]
    fn test_image_new_hex_color() {
        let result = image_namespace_method(
            "new",
            &[Value::Int(10), Value::Int(10), Value::string("#FF0000")],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_image_dimensions() {
        let img = image_namespace_method("new", &[Value::Int(200), Value::Int(150)]).unwrap();
        if let Value::Image(img_ref) = img {
            let result = image_method(&img_ref, "dimensions", &[]);
            assert!(result.is_ok());
            if let Value::List(dims) = result.unwrap() {
                let dims = dims.borrow();
                assert_eq!(dims[0], Value::Int(200));
                assert_eq!(dims[1], Value::Int(150));
            }
        }
    }

    #[test]
    fn test_image_resize() {
        let img = image_namespace_method("new", &[Value::Int(100), Value::Int(100)]).unwrap();
        if let Value::Image(img_ref) = img {
            let result = image_method(&img_ref, "resize", &[Value::Int(50), Value::Int(50)]);
            assert!(result.is_ok());
            if let Value::Image(resized) = result.unwrap() {
                assert_eq!(resized.width(), 50);
                assert_eq!(resized.height(), 50);
            }
        }
    }

    #[test]
    fn test_image_rotate() {
        let img = image_namespace_method("new", &[Value::Int(100), Value::Int(50)]).unwrap();
        if let Value::Image(img_ref) = img {
            let result = image_method(&img_ref, "rotate", &[Value::Int(90)]);
            assert!(result.is_ok());
            if let Value::Image(rotated) = result.unwrap() {
                // Dimensions should be swapped after 90 degree rotation
                assert_eq!(rotated.width(), 50);
                assert_eq!(rotated.height(), 100);
            }
        }
    }

    #[test]
    fn test_image_grayscale() {
        let img = image_namespace_method(
            "new",
            &[Value::Int(10), Value::Int(10), Value::string("red")],
        )
        .unwrap();
        if let Value::Image(img_ref) = img {
            let result = image_method(&img_ref, "grayscale", &[]);
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), Value::Image(_)));
        }
    }

    #[test]
    fn test_image_color_operations() {
        let img = image_namespace_method("new", &[Value::Int(10), Value::Int(10)]).unwrap();
        if let Value::Image(img_ref) = img {
            assert!(image_method(&img_ref, "brightness", &[Value::Float(0.5)]).is_ok());
            assert!(image_method(&img_ref, "contrast", &[Value::Float(1.5)]).is_ok());
            assert!(image_method(&img_ref, "blur", &[Value::Float(1.0)]).is_ok());
            assert!(image_method(&img_ref, "sharpen", &[]).is_ok());
        }
    }

    #[test]
    fn test_image_to_bytes() {
        let img = image_namespace_method("new", &[Value::Int(5), Value::Int(5)]).unwrap();
        if let Value::Image(img_ref) = img {
            let result = image_method(&img_ref, "to_bytes", &[Value::string("png")]);
            assert!(result.is_ok());
            if let Value::List(bytes) = result.unwrap() {
                assert!(!bytes.borrow().is_empty());
            }
        }
    }

    #[test]
    fn test_image_dispatch() {
        let result = dispatch_namespace_method("Image", "new", &[Value::Int(10), Value::Int(10)]);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::Image(_)));
    }

    // ============================================================================
    // Ref Module Tests (Weak References)
    // ============================================================================

    #[test]
    fn test_ref_weak_creates_weak_ref() {
        let list = Value::list(vec![Value::Int(1), Value::Int(2)]);
        let result = ref_method("weak", &[list]);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::WeakRef(_)));
    }

    #[test]
    fn test_ref_weak_with_map() {
        let map = Value::empty_map();
        let result = ref_method("weak", &[map]);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::WeakRef(_)));
    }

    #[test]
    fn test_ref_weak_with_set() {
        let set = Value::empty_set();
        let result = ref_method("weak", &[set]);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::WeakRef(_)));
    }

    #[test]
    fn test_ref_weak_rejects_non_container() {
        // Should fail with Int
        let result = ref_method("weak", &[Value::Int(42)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires a container type"));

        // Should fail with String
        let result = ref_method("weak", &[Value::string("hello")]);
        assert!(result.is_err());
    }

    #[test]
    fn test_ref_upgrade_alive() {
        let list = Value::list(vec![Value::Int(42)]);
        let weak = ref_method("weak", &[list.clone()]).unwrap();

        // Upgrade should return the original list
        let result = ref_method("upgrade", &[weak]);
        assert!(result.is_ok());
        let upgraded = result.unwrap();
        assert!(matches!(upgraded, Value::List(_)));
    }

    #[test]
    fn test_ref_upgrade_dead() {
        // Create weak ref in a scope, then drop the strong ref
        let weak = {
            let list = Value::list(vec![Value::Int(1)]);
            ref_method("weak", &[list]).unwrap()
        };

        // Upgrade should return Null
        let result = ref_method("upgrade", &[weak]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);
    }

    #[test]
    fn test_ref_is_alive_true() {
        let list = Value::list(vec![Value::Int(1)]);
        let weak = ref_method("weak", &[list.clone()]).unwrap();

        let result = ref_method("is_alive", &[weak]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_ref_is_alive_false() {
        let weak = {
            let list = Value::list(vec![Value::Int(1)]);
            ref_method("weak", &[list]).unwrap()
        };

        let result = ref_method("is_alive", &[weak]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_ref_dispatch() {
        let list = Value::list(vec![Value::Int(1)]);
        let result = dispatch_namespace_method("Ref", "weak", &[list]);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::WeakRef(_)));
    }

    #[test]
    fn test_ref_unknown_method() {
        let result = ref_method("unknown", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no method 'unknown'"));
    }

    #[test]
    fn test_weak_ref_method_upgrade() {
        let list = Value::list(vec![Value::Int(42)]);
        if let Some(Value::WeakRef(weak)) = list.weak_ref() {
            let result = weak_ref_method("upgrade", &[], &weak);
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), Value::List(_)));
        } else {
            panic!("Expected WeakRef");
        }
    }

    #[test]
    fn test_weak_ref_method_is_alive() {
        let list = Value::list(vec![Value::Int(42)]);
        if let Some(Value::WeakRef(weak)) = list.weak_ref() {
            let result = weak_ref_method("is_alive", &[], &weak);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Value::Bool(true));
        } else {
            panic!("Expected WeakRef");
        }
    }

    #[test]
    fn test_weak_ref_method_target_type() {
        let list = Value::list(vec![Value::Int(42)]);
        if let Some(Value::WeakRef(weak)) = list.weak_ref() {
            let result = weak_ref_method("target_type", &[], &weak);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Value::string("List"));
        } else {
            panic!("Expected WeakRef");
        }
    }

    #[test]
    fn test_weak_ref_method_target_type_map() {
        let map = Value::empty_map();
        if let Some(Value::WeakRef(weak)) = map.weak_ref() {
            let result = weak_ref_method("target_type", &[], &weak);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Value::string("Map"));
        } else {
            panic!("Expected WeakRef");
        }
    }
}
