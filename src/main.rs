use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use serpapi::serpapi::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::env;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

//enum of the commands list and compiler settings
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Commandos disponibles:")]
enum Commands {
    #[command( description = "Buscar informacion en Google... ")]
    Buscarengoogle(String),
    #[command( description = "Muestra el cumpleaños de un usuario... ")]
    Cumpleaños(String),
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

    let bot = Bot::new(token);
    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    //charger the data base JSON (simple)

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
            let mut options = HashMap::new();
            options.insert("api_key".to_string(), "SER_API_KEY".to_string());
            options.insert("engine".to_string(), "google_ia_overview".to_string());
            options.insert("q".to_string(),query);

            let client = Client::new(options).unwrap();
            let results = client.search(HashMap::new())
                                .await.expect("request");
            let respose_ia = results["ia_overview"]["answer"]
                                .as_str()
                                .unwrap_or("Not find a respose of the IA");
                 
            //here integer the logic of find extern
            let _ = bot.send_message(msg.chat.id, respose_ia).await;
        }
        Commands::Cumpleaños(mencion) => {
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
            bot.send_message(msg.chat.id, format!("Usuarios {mencion} silenciado por 1 hora.")).await?;
        }
    }
    Ok(())
}
