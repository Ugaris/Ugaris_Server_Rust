use async_trait::async_trait;
use sqlx::PgPool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaServerRecord {
    pub area_id: i32,
    pub mirror_id: i32,
    pub server_addr: i32,
    pub server_port: i32,
    pub online: bool,
}

#[async_trait]
pub trait AreaRepository: Send + Sync {
    async fn mark_alive(
        &self,
        area_id: i32,
        mirror_id: i32,
        server_addr: i32,
        server_port: i32,
    ) -> anyhow::Result<()>;
    async fn mark_down(&self, area_id: i32, mirror_id: i32) -> anyhow::Result<()>;
    async fn get_area(
        &self,
        area_id: i32,
        mirror_id: i32,
    ) -> anyhow::Result<Option<AreaServerRecord>>;
}

#[derive(Debug, Clone)]
pub struct PgAreaRepository {
    pool: PgPool,
}

impl PgAreaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AreaRepository for PgAreaRepository {
    async fn mark_alive(
        &self,
        area_id: i32,
        mirror_id: i32,
        server_addr: i32,
        server_port: i32,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "insert into area_servers(area_id, mirror_id, server_addr, server_port, online, last_seen) \
             values ($1, $2, $3, $4, true, now()) \
             on conflict (area_id, mirror_id) do update set \
             server_addr = excluded.server_addr, server_port = excluded.server_port, online = true, last_seen = now()",
        )
        .bind(area_id)
        .bind(mirror_id)
        .bind(server_addr)
        .bind(server_port)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_down(&self, area_id: i32, mirror_id: i32) -> anyhow::Result<()> {
        sqlx::query("update area_servers set online = false, last_seen = now() where area_id = $1 and mirror_id = $2")
            .bind(area_id)
            .bind(mirror_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_area(
        &self,
        area_id: i32,
        mirror_id: i32,
    ) -> anyhow::Result<Option<AreaServerRecord>> {
        let row = sqlx::query_as::<_, (i32, i32, i32, i32, bool)>(
            "select area_id, mirror_id, server_addr, server_port, online from area_servers where area_id = $1 and mirror_id = $2",
        )
        .bind(area_id)
        .bind(mirror_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(area_id, mirror_id, server_addr, server_port, online)| AreaServerRecord {
                area_id,
                mirror_id,
                server_addr,
                server_port,
                online,
            },
        ))
    }
}
