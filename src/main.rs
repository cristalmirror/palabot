use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use serpapi::serpapi::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::env;
use serde::Deserialize;
use chrono::{Local, Datelike, Duration as ChronoDuration};

use std::time::Duration;


pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Deserialize)]
struct User {
    name: String,
    birthday: String,
    chat_id: i64,
}

#[derive(Deserialize)]
struct Database{
    users: Vec<User>,//list of users
}

//enum of the commands list and compiler settings
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Commandos disponibles:")]
enum Commands {
    #[command( description = "Buscar informacion en Google... ")]
    Buscarengoogle(String),
    #[command( description = "Muestra el cumpleaños de un usuario... ")]
    Cumpleanios(String),
    #[command( description = "Silencia a un usuario por una hora (Solo Admins). ")]
    Bloqueo(String),

}
//Structure to own DB json
type Db = Arc<Mutex<HashMap<String,String>>>;

//main function with tokio traits
#[tokio::main]
async fn main() -> Result<(), Error> {
    //collect the arguments to execute the token
    let args: Vec<String> = env::args().collect();

    let token = if args.len() > 1 {
        args[1].clone()
    } else {
        env::var("TELOXIDE_TOKEN").expect("[SERVER] Error: Debes pasar el token como parámetro (./tsbpal TOKEN) o configurar TELOXIDE_TOKEN")
    };
    
    let bot = Bot::with_client(token, teloxide::net::client_from_env());
    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    //charger the data base JSON (simple)
    let bot_for_scheduler = bot.clone();
    tokio::spawn(async move {
        start_birtday_scheduler(bot_for_scheduler).await;
    });

    let handler = Update::filter_message()
        .filter_command::<Commands>()
        .endpoint(answer);
    
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db])
        .build()
        .dispatch()
        .await;
    Ok(())
}

async fn answer(
    bot: Bot,
    msg: Message, 
    cmd: Commands, 
    db: Db,) -> Result<(), Error> {
    
    match cmd {
        Commands::Buscarengoogle(query) => {
            let args: Vec<String> = env::args().collect();
            let mut options = HashMap::new();
            options.insert("api_key".to_string(), args[2].clone());
            options.insert("engine".to_string(), "google".to_string());
            options.insert("q".to_string(),query);

            let client = Client::new(options).unwrap();
            let results = client.search(HashMap::new())
                                .await.expect("request");
            println!("Resultado JSON: {}", serde_json::to_string_pretty(&results).unwrap());
            if let Some(references) = results["ai_overview"]["snippet"].as_array() {//trying catch the IA reference
                if !references.is_empty() {
                    let respose_ia = references[0]["snippet"]          
                        .as_str()
                        .unwrap_or("No se encontró una respuesta de la IA.");               
                   let _ = bot.send_message(msg.chat.id, respose_ia).await;
                } else {
                   let _ = bot.send_message(msg.chat.id, "No se encontraron referencias.").await;
                }
            } else if let Some(first_result) = results["organic_results"].as_array().and_then(|a| a.get(0)) {//if haven't ia snippet in the JSON respose
                let title = first_result["title"].as_str().unwrap_or("Sin título");
                let link = first_result["link"].as_str().unwrap_or("");
                let snippet = first_result["snippet"].as_str().unwrap_or("");
        
                let respuesta = format!("🌐 Resultado principal:\n\n**{}**\n{}\n\n{}", title, snippet, link);
                bot.send_message(msg.chat.id, respuesta).await?;
            } else {
                bot.send_message(msg.chat.id, "Google no proporcionó referencias de IA.").await?;
            }
        }
        Commands::Cumpleanios(mencion) => {
            let respuesta = {
                let data = db.lock().expect("Error in mutex");
                    data.get(&mencion)
                        .map(|fecha| format!("el cumpleaños {mencion} es el {fecha}"))
                        .unwrap_or_else(|| "Usuario no encontrado en la base de datos JSON".to_string())
                };
           let _ = bot.send_message(msg.chat.id, respuesta).await;
        }
        Commands::Bloqueo(mencion) => {
            //La logica de bloqueo requiere verificar permisos de admin
            //Teloxide maneja estas peticiones a la API de telegram [4]
           
        }
    }
    Ok(())
}

async fn check_birthdays(bot: Bot) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //catch the date today formated 
    let now = Local::now();
    let today = format!("{:02}-{:02}", now.day(),now.month());

    //read the archive in JSON database
    let data = std::fs::read_to_string("db.json")?;
    //we use serder JSON to interprete the structure
    let db: Database = serde_json::from_str(&data)?;

    for user in db.users {
        if user.birthday == today {
            //send message
           bot.send_message(
                ChatId(user.chat_id), 
                format!("¡Feliz Cumpleaños {}! Toda la palabanda está de fiesta.", user.name)
            ).await?;
        }
    }
    Ok(())
}

async fn start_birtday_scheduler(bot: Bot) {
    loop {
        let now = Local::now();

        //we calculated the midnigth of the next day
        let next_run = (now + ChronoDuration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        
        let duration_until_midnight = (next_run - now).to_std().unwrap();
        //the bot waiting the asynchronus form without block other functions
        tokio::time::sleep(duration_until_midnight).await;

        if let Err(e) = check_birthdays(bot.clone()).await {
            log::error!("Error en la tarea de Cumleaños: {}",e);
        }
    }
}