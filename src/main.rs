use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use serpapi::serpapi::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::env;
use serde::Deserialize;
use chrono::{Local, Datelike, Duration as ChronoDuration};




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
    #[command( description = "Instructivo de uso del Palabot...")]
    Start,

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

    /*charger the data base JSON (simple)
        this operation don't stop the thread
        in the main fucion scope
    */
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
    ) -> Result<(), Error> {
    
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
            let title_init_result = String::from("🌐Resultados de la Busqueda\n\n");
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
            } else if let Some(array) = results["organic_results"].as_array() {//if haven't ia snippet in the JSON respose
                bot.send_message(msg.chat.id,title_init_result).await?;
                for res in array {//select element of the results
                    let title = res["title"].as_str().unwrap_or("Sin título");
                    let link = res["link"].as_str().unwrap_or("");
                    let snippet = res["snippet"].as_str().unwrap_or("");

                     bot.send_message(msg.chat.id,format!(//mesagges results
                        "{}\n\n{}\n\n{}\n\n",
                        link, title, snippet
                    )).await?;
                }
                
                
            } else {
                bot.send_message(msg.chat.id, "Google no proporcionó referencias de IA.").await?;
            }
        }
        Commands::Cumpleanios(mencion) => {
            println!("[BOT]: cumpleanios init");
            //read the db archive
            let data = std::fs::read_to_string("src/db.json").expect("[BOT]: can't read the DB of the bot");
            println!("[BOT]: db has read");
            //interprete the content of json with serde
            let db: Database = match serde_json::from_str(&data) {
                Ok(val) => val,
                Err(e) => {
                    println!("[BOT] ERROR de formato en JSON: {}", e);
                    return Ok(()); // Salimos sin que el bot muera
                }
            };
            println!("[BOT]: db has ported to Rust code");
            //find the user in the list 
            let mut finded = false; 
            for user in db.users {
                if user.name == mencion {
                    bot.send_message(msg.chat.id, format!("🎂 El cumpleaños de {} es el {}.", user.name, user.birthday)).await?;
                    println!("[BOT]: El cumpleaños de {} es el {}.", user.name, user.birthday);
                    finded = true;
                    break;
                }
            }
            if finded {
                println!("[BOT]: user is finded fine");
            }

            //if the name not exist in the data base send error messages
            if !finded {
                bot.send_message(msg.chat.id, format!("❌ No encontré a {} en mi base de datos.", mencion)).await?;
                println!("[BOT]: No encontré a {} en mi base de datos.", mencion);
            }
        }
        Commands::Bloqueo(mencion) => {
            let chat_id = msg.chat.id;
            //catch who send the command
            let sender = match msg.from() {
                Some(u) => u,
                None => {
                    bot.send_message(chat_id,"No se identificar al remimitente.").await?;
                    return Ok(());
                }
            };

            //verify that the sender is admin or owner
            let sender_member = bot.get_chat_member(chat_id, send.id).await?;
            let is_admin = matches!(
                sender_member.status(),
                teloxide::types::ChatMemberStatus::Adiministrator | teloxide::types::ChatMemberStatus::Owner
            );

            //parse arguments first token = @user, second (option) = minutes
            let mut parts = mencion.spit=whitespace();
            let username_raw = match parts.next() {
                Some(u) => u.trim_start_matches('@').to_string(),
                None => {
                    bot.send_message(chat_id, "⚠️ Uso: /bloqueo @usuario [minutos]").await?;
                    return Ok(());
                }
            };

            let (target_id, target_name) = match target {
                Some(t)=> t,
                None => {
                    bot.send_message(chat_id,"❌ No pude encontrar a ese usuario. Prueba respondiendo su mensaje.").await?;
                    return Ok(());
                }
            };

            //can't block others admins
            let target_member = bot.get_chat_member(chat_id, target_id).await?;
            if matches!(
                target_member.status(),
                teloxide::types::ChatMemberStatus::Adiministrator | teloxide::types::ChatMemberStatus::Owner
            ) {
                bot.send_message(chat_id,"❌ No puedes silenciar a un administrador.").await?;
                return Ok(());
            }

            //calculation timeestamp of expirations
            let until_ts = std::time::System::now()
                .duration_since()
                .unwrap()
                .as_secs() as i64 + (minutes * 60) as i64;

            bot.restrict_chat_member(chat_id, target_id, teloxide::types::empty())
                .until_date(until_ts)
                .await?;

            bot.send_message(chat_id,format!("🔇 @{target_name} silenciado por {minutes} minuto(s).\na casa papu!!")).await?;
            
            //Teloxide maneja estas peticiones a la API de telegram [4]
            bot.send_message(msg.chat.id, format!("Usuario Bloqueado: {} (Operacion aun no implementada)", mencion)).await?;
        }

        Commands::Start => {
             bot.send_message(msg.chat.id, format!("  🪏Bienvenido al Palabot🪏🪏Coamndos:
            \n\n🪏`/start` Muestra la lista de comandos del palabot
            \n🪏`/buscarengoogle <texto de la busqueda>`este comando arroja el primer resultado de la busqueda en google, permite maximo 256 busquedas al mes
            \n🪏`/cumpleanios <@usuario>` Permite arrojar la fecha de cumpleaños del usuario mencionado
            \n🪏`/bloque <@usuario> <1hs/1min/etc>` (operacion no implementada) permitira a los admins silenciar a un usuario por un tiempo predeterminado
            \n🪏Gracias a todos los usuarios de Palabot:\n🪏repositorio: https://github.com/cristalmirror/palabot ")).await?;
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
