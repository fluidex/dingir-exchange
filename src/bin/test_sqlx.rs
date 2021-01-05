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

struct IterHelper<T1, T2> (T1, T2, (i32, Option<i32>));

impl<T1, T2> Iterator for IterHelper<T1, T2>
where T1: Iterator<Item=i32>, T2: Iterator<Item=i32>,
{
    type Item = Option<i32>;
    fn next(&mut self) -> Option<Self::Item>
    {
        self.2.0 = self.2.0 + 1;       
        let ret = match self.2.1 {
            Some(i) => {
                if i > self.2.0 {
                    self.0.next()
                }else{
                    self.2.1 = self.1.next();
                    return Some(None);
                }
            },
            None => self.0.next(), 
        };
        match ret {
            Some(i) => Some(Some(i)),
            None => None,
        }        
    }
}

fn expand_argsn(inp : (i32, Vec<i32>)) -> Vec<Option<i32>>
{
    let t1 = 1..(inp.0+1);
    let mut t2 = inp.1.into_iter();
    let init = t2.next();
    IterHelper(t1, t2, (0, init)).collect()
}

//extend table schemas for fromrow macro
trait TableSchemas : 'static
{
    const ARGN: i32;
    fn table_name() -> &'static str;
    fn default_argsn() -> Vec<i32>{ vec![] }
}

trait FinalQuery
{
    fn query_final<T : sqlx::Done>(res: Result::<T, sqlx::Error>) -> SimpleResult<()>;
}

trait BindQueryArg<'a, DB> : Sized 
where DB: sqlx::Database
{
    //PgArguments sealed its add method as pub(crate) so we can not use the concerete type
    //directly (or apply the more cumbersome "as" statement ...)    
    fn bind_args<'g, 'q : 'g>(&'q self, arg : &mut impl sqlx::Arguments<'g, Database = DB>) where 'a : 'q;
}

trait CommonSQLQuery<DB: sqlx::Database> : 'static
{
    fn sql_statement<T: TableSchemas>() -> String;
}

trait CommonSQLQueryWithBind<DB> : CommonSQLQuery<DB> where DB: sqlx::Database
{
    type DefaultArg: for<'r> sqlx::Arguments<'r, Database = DB> + for<'r> sqlx::IntoArguments<'r, DB>;
    //re-export, for compiler seems not very clear about Self::QueryType::DefaultArg?
    fn new() -> Self::DefaultArg
    {
        <Self::DefaultArg as Default>::default()
    }
}

impl<U> CommonSQLQueryWithBind<sqlx::Postgres> for U where U : CommonSQLQuery<sqlx::Postgres>
{
    type DefaultArg = sqlx::postgres::PgArguments;
}


#[tonic::async_trait]
trait CommonSqlxAction<DB> : CommonSQLQueryWithBind<DB> + FinalQuery
where DB: sqlx::Database
{
    async fn sql_query<'c, 'a, Q, C>(qr :Q, conn: C) -> SimpleResult<()>
    where
        C: sqlx::Executor<'c, Database = DB>,
        Q: TableSchemas + BindQueryArg<'a, DB> + Send,
    {
        let mut arg : Self::DefaultArg = Default::default();
        qr.bind_args(&mut arg);

        let ret = sqlx::query_with(&Self::sql_statement::<Q>(), arg)
            .execute(conn).await;
        Self::query_final(ret)
    }
}



#[tonic::async_trait]
trait SqlxAction<'a, QT, DB> : TableSchemas + Sized + BindQueryArg<'a, DB>
where 
    QT: CommonSQLQueryWithBind<DB> + FinalQuery,
    DB: sqlx::Database,
{
    async fn sql_query<'c, C>(&'a self, conn: C) -> SimpleResult<()>
    where
        C: sqlx::Executor<'c, Database = DB>,
    {
        let mut arg : QT::DefaultArg = Default::default();
        self.bind_args(&mut arg);

        let sqlstate = QT::sql_statement::<Self>();
        let ret = sqlx::query_with(&sqlstate, arg)
            .execute(conn).await;
            QT::query_final(ret)
    }
}

struct InsertTable {}

impl FinalQuery for InsertTable
{
    fn query_final<T : sqlx::Done>(res: Result::<T, sqlx::Error>) -> SimpleResult<()>
    {
        let maydone = res?;
        if maydone.rows_affected() != 1 {
            return Err(anyhow::anyhow!("Insert has no effect"));
        }
        Ok(())
    }
}

impl CommonSQLQuery<sqlx::Postgres> for InsertTable
{
    fn sql_statement<T: TableSchemas>() -> String
    {
        //not good, sql statements can be cached or formed by marco in advance 
        //fix it later ...
        format!("INSERT INTO {} VALUES ({})",
            T::table_name(),
            expand_argsn((T::ARGN, T::default_argsn()))
                .iter()
                .map(|i| match i {
                    Some(i) => format!("${}", i),
                    None => String::from("DEFAULT"),
                })
                .fold(String::new(), |acc,s|{if acc.len() == 0 {s} else{acc+","+&s}})
        )
    }
}

#[cfg(test)]
mod tests {

    use crate::expand_argsn;

    fn gen_insert_str(i: (i32, Vec<i32>)) -> String
    {
        expand_argsn(i)
            .iter()
            .map(|i| match i {
                Some(i) => format!("${}", i),
                None => String::from("DEFAULT"),
            })
            .fold(String::new(), |acc,s|{if acc.len() == 0 {s} else{acc+","+&s}})     
    }

    #[test]
    fn table_schema_args() {
        assert_eq!(gen_insert_str((4, vec![])), "$1,$2,$3,$4");
        assert_eq!(gen_insert_str((4, vec![3])), "$1,$2,DEFAULT,$3,$4");
        assert_eq!(gen_insert_str((4, vec![1])), "DEFAULT,$1,$2,$3,$4");
        assert_eq!(gen_insert_str((4, vec![1,6,7])), "DEFAULT,$1,$2,$3,$4,DEFAULT,DEFAULT");
        assert_eq!(gen_insert_str((1, vec![1,2,3])), "DEFAULT,DEFAULT,DEFAULT,$1");
        assert_eq!(gen_insert_str((1, vec![1,2,4])), "DEFAULT,DEFAULT,$1,DEFAULT");
    }
}

impl TableSchemas for OperationLog
{
    const ARGN: i32 = 4;
    fn table_name() -> &'static str {"operation_log"}
}

impl BindQueryArg<'_, sqlx::Postgres> for OperationLog
{
    fn bind_args<'g, 'q : 'g>(&'q self, arg : &mut impl sqlx::Arguments<'g, Database = sqlx::Postgres>)
    {
        arg.add(self.id);
        arg.add(self.time);
        arg.add(&self.method);
        arg.add(&self.params);
    }

    // fn sql_statement(&self) -> String 
    // {
    //     println!("overload");
    //     InsertTable::sql_statement(self)
    // }    
}

impl CommonSqlxAction<sqlx::Postgres> for InsertTable{}
impl SqlxAction<'_, InsertTable, sqlx::Postgres> for OperationLog{}

// #[tonic::async_trait]
// impl InsertTable<'_> for OperationLog
// {
//     async fn do_insert(&self, conn: &mut PgConnection) -> SimpleResult<()>
//     {
//         execute_final(sqlx::query!(
//                 //this is not work ...
//                 //std::format!("INSERT INTO {} VALUES ($1, $2, $3, $4)", "operation_log"),
//                 "INSERT INTO operation_log VALUES ($1, $2, $3, $4)",
//                 self.id, self.time, &self.method, &self.params
//             ).execute(conn).await)
//     }
// }

/*
#[tokio::main]
async fn main() -> SimpleResult<()>{
    let mut conn = PgConnection::connect("postgres://exchange:exchange_AA9944@172.30.41.204/exchange").await?;

    let curr_time = chrono::Local::now();

    let log = OperationLog {
        id: curr_time.timestamp(),
        time: curr_time.naive_local(),
        method: String::from("SQLTest insert"),
        params: String::from("No parameters"),
    };

//    InsertTable::sql_action(&log, &mut conn).await?;
    log.sql_query(&mut conn).await?;

    let fetchLog = sqlx::query_as!(
        OperationLogPartial,
        "SELECT time, method, params FROM operation_log WHERE id = $1",
        log.id
    ).fetch_one(&mut conn)
    .await?;

    println!("{:?}", fetchLog);

    Ok(())
}
*/
#[cfg(sqlxverf)]
fn test1()
{
    sqlx::query!("insert into operation_log select $1, $2, $3, $4");
}

fn main(){
    let mut rt: tokio::runtime::Runtime = tokio::runtime::Builder::new()
        .enable_all()
        .basic_scheduler()
        .build()
        .expect("build runtime");
    
    let mut conn = rt.block_on(
        PgConnection::connect("postgres://exchange:exchange_AA9944@172.30.41.204/exchange")).unwrap();

    let curr_time = chrono::Local::now();

    let log = OperationLog {
        id: curr_time.timestamp(),
        time: curr_time.naive_local(),
        method: String::from("SQLTest insert"),
        params: String::from("No parameters"),
    };

    let qid = log.id;
    let mut logv = vec![log];
    for l in logv.drain(0..) {
        //rt.block_on(l.sql_query(&mut conn)).unwrap();
        rt.block_on(InsertTable::sql_query(l, &mut conn)).unwrap();
    }

    let pr1 : Vec<i64> = vec![qid+1,qid+2];
    let pr2 : Vec<chrono::NaiveDateTime> = vec![curr_time.naive_local(),curr_time.naive_local()];
    let pr3 : Vec<&str> = vec!["SQLTest insert","SQLTest insert"];
    let pr4 : Vec<&str> = vec!["Add1","Add2"];

    {
        // let f = sqlx::query("insert into operation_log values ($1,$3,$5,$7),($2,$4,$6,$8)")
        // .bind(qid+1).bind(qid+2).bind(curr_time.naive_local()).bind(curr_time.naive_local())
        // .bind("SQLTest insert").bind("SQLTest insert").bind("Add1").bind("Add2")
        //.bind(qid+1).bind(qid+2).bind(curr_time.naive_local()).bind(curr_time.naive_local())
        //.bind("SQLTest insert").bind("SQLTest insert").bind("Add1").bind("Add2")   
        let f = sqlx::query("insert into operation_log select $1, $2, $3, $4")
        .bind(pr1).bind(pr2).bind(pr3).bind(pr4)
        .execute(&mut conn);
        let done = rt.block_on(f).expect("batch insert");
        println!("{}", done.rows_affected());
    }
    

    let query = format!("SELECT time, method, params FROM {} WHERE id = $1", "operation_log");
    let fetch_log : OperationLogPartial = rt.block_on(sqlx::query_as(&query)
        .bind(qid)
        .fetch_one(&mut conn)).unwrap();

    println!("{:?}", fetch_log);    
}