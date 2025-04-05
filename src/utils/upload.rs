use anyhow::Result;
use object_store::{aws::AmazonS3, path::Path, ObjectStore, WriteMultipart};
use sha256::digest;
use sqlx::{query, PgConnection};

pub async fn upload(conn: &mut PgConnection, s3: &AmazonS3, bytes: Vec<u8>) -> Result<i32> {
    let sha = digest(&*bytes);

    let path = Path::from(format!("/{}/{}/{}", &sha[..2], &sha[2..4], &sha[4..]));
    let file = query!("SELECT * FROM files WHERE hash = $1", sha)
        .fetch_optional(&mut *conn)
        .await?;
    if let Some(file) = file {
        return Ok(file.id);
    };

    let len = i32::try_from(bytes.len())?;
    // 5MiB
    if len > 5 * 1024 * 1024 {
        let upload = s3.put_multipart(&path).await?;
        let mut write = WriteMultipart::new(upload);
        write.write(&bytes);
        write.finish().await?;
    } else {
        s3.put(&path, bytes.into()).await?;
    }
    let resp = query!(
        "INSERT INTO files (hash, size) VALUES ($1, $2) RETURNING id",
        sha,
        len
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(resp.id)
}
