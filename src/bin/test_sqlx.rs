use anyhow::Result as SimpleResult;
use sqlx::prelude::*;
use sqlx::postgres::PgConnection;
use sqlx::types::chrono;

#[derive(sqlx::FromRow)]
pub struct OperationLog {
    pub id: i64,
    pub time: chrono::NaiveDateTime,
    pub method: String,
    pub params: String,
}

#[derive(sqlx::FromRow, Debug)]
pub struct OperationLogPartial {
    pub time: chrono::NaiveDateTime,
    pub method: String,
    pub params: String,
}

fn execute_final<T : sqlx::Done>(res: Result::<T, sqlx::Error>) -> SimpleResult<()>
{
    let maydone = res?;
    if maydone.rows_affected() != 1 {
        return Err(anyhow::anyhow!("Insert has no effect"));
    }
    Ok(())
}

#[tonic::async_trait]
trait InsertTable<'a, DB = sqlx::Postgres> : sqlx::FromRow<'a, DB::Row>
where 
    DB : sqlx::Database 
{
    async fn do_insert(&self, conn: &mut DB::Connection) -> SimpleResult<()>;
}



#[tonic::async_trait]
impl InsertTable<'_> for OperationLog
{
    async fn do_insert(&self, conn: &mut PgConnection) -> SimpleResult<()>
    {
        execute_final(sqlx::query!(
                //this is not work ...
                //std::format!("INSERT INTO {} VALUES ($1, $2, $3, $4)", "operation_log"),
                "INSERT INTO operation_log VALUES ($1, $2, $3, $4)",
                self.id, self.time, &self.method, &self.params
            ).execute(conn).await)
    }
}

#[tokio::main]
async fn main() -> SimpleResult<()>{
    let mut conn = PgConnection::connect("postgres://exchange:exchange_AA9944@127.0.0.1/exchange").await?;

    let curr_time = chrono::Local::now();

    let log = OperationLog {
        id: curr_time.timestamp(),
        time: curr_time.naive_local(),
        method: String::from("SQLTest insert"),
        params: String::from("No parameters"),
    };

    log.do_insert(&mut conn).await?;

    let fetchLog = sqlx::query_as!(
        OperationLogPartial,
        "SELECT time, method, params FROM operation_log WHERE id = $1",
        log.id
    ).fetch_one(&mut conn)
    .await?;

    println!("{:?}", fetchLog);

    Ok(())
}
