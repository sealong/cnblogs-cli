use std::fs::File;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::models::user::UserInfo;

const CACHE_DIR: &str = ".cnblogs";
const CACHE: &str = "token";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Cache {
    pub id: u64,
    pub blog_id: u64,
    pub blog_app: String,
    pub username: String,
    pub token: String,
}

impl Cache {
    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(buf)?)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// 更新用户字段，保留已有 token。
    pub fn apply_user(&mut self, user: &UserInfo) {
        self.id = user.account_id;
        self.blog_id = user.blog_id;
        self.blog_app = user.blog_app.clone();
        self.username = user.display_name.clone();
    }

    /// 检查 token 是否为空
    pub fn is_token_empty(&self) -> bool {
        self.token.trim().is_empty()
    }

    /// 验证缓存数据的有效性
    pub fn is_valid(&self) -> bool {
        !self.is_token_empty() && self.id > 0
    }
}

impl From<UserInfo> for Cache {
    fn from(value: UserInfo) -> Self {
        let mut cache = Self::default();
        cache.apply_user(&value);
        cache
    }
}

#[derive(Debug)]
pub struct CacheDir {
    pub cache_dir: PathBuf,
    pub cache_file: PathBuf,
    pub home_dir: PathBuf,
}

impl CacheDir {
    pub fn new() -> Result<Self> {
        let home_dir =
            home::home_dir().ok_or_else(|| anyhow!("无法获取用户家目录，退出。".red()))?;
        let cache_dir = PathBuf::from(CACHE_DIR);
        let cache_file = PathBuf::from(CACHE);

        Ok(Self {
            cache_dir,
            cache_file,
            home_dir,
        })
    }

    /// 初始化，检查文件夹和目录是否存在，如果不存在则创建
    pub fn init(&self) -> Result<()> {
        self.ensure_dir()?;
        self.ensure_file()
    }

    /// 获取缓存目录的完整路径
    pub fn full_cache_dir(&self) -> PathBuf {
        self.home_dir.join(&self.cache_dir)
    }

    /// 获取缓存文件的完整路径
    pub fn full_cache_file(&self) -> PathBuf {
        self.full_cache_dir().join(&self.cache_file)
    }

    /// 检查缓存目录是否存在，不存在创建。
    pub fn ensure_dir(&self) -> Result<()> {
        let p = self.full_cache_dir();
        if !p.exists() {
            fs::create_dir_all(p)?;
        }
        Ok(())
    }

    /// 检查缓存文件是否存在，不存在创建。
    pub fn ensure_file(&self) -> Result<()> {
        let p = self.full_cache_file();
        if !p.exists() {
            fs::File::create(p)?;
        }
        Ok(())
    }

    /// 写入缓存文件
    pub fn write(&self, buf: &[u8]) -> Result<()> {
        let mut f = File::create(self.full_cache_file())?;
        f.write_all(buf)?;
        Ok(())
    }

    /// 读取缓存文件
    pub fn read(&self) -> Result<Vec<u8>> {
        let mut buf = vec![];
        let mut f = File::open(self.full_cache_file())?;
        f.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user() -> UserInfo {
        UserInfo {
            user_id: "u".into(),
            space_user_id: 1,
            account_id: 42,
            blog_id: 7,
            display_name: "alice".into(),
            face: String::new(),
            avatar: String::new(),
            seniority: String::new(),
            blog_app: "alice-blog".into(),
            following_count: 0,
            follower_count: 0,
            is_vip: false,
            joined: String::new(),
        }
    }

    #[test]
    fn apply_user_preserves_existing_token() {
        let mut cache = Cache {
            token: "pat-secret".into(),
            id: 1,
            ..Cache::default()
        };
        cache.apply_user(&sample_user());

        assert_eq!(cache.token, "pat-secret");
        assert_eq!(cache.id, 42);
        assert_eq!(cache.blog_id, 7);
        assert_eq!(cache.blog_app, "alice-blog");
        assert_eq!(cache.username, "alice");
    }

    #[test]
    fn status_cache_roundtrip_keeps_token() {
        let mut cache = Cache {
            token: "pat-secret".into(),
            ..Cache::default()
        };
        cache.apply_user(&sample_user());

        let loaded = Cache::from_bytes(&cache.to_bytes().unwrap()).unwrap();
        assert_eq!(loaded.token, "pat-secret");
        assert_eq!(loaded.id, 42);
        assert!(loaded.is_valid());
    }

    #[test]
    fn from_user_info_does_not_invent_token() {
        let cache = Cache::from(sample_user());
        assert!(cache.is_token_empty());
        assert_eq!(cache.id, 42);
    }
}
