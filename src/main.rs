use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use serpapi::serpapi::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;


//enum of the commands list and compiler settings
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Commandos disponibles:")]
enum Commands {
    #[command( description = "Buscar informacionen Google... ")]
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
async fn main() {
    let bot = Bot::from_env();
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

}

async fn answer(
    bot: Bot,
    msg: Message, 
    cmd: Commands, 
    db: Db,) -> ResponseResult<()> {
    
    match cmd {
        Commands::Buscarengoogle(query) => {
            let mut options = HashMap::new();
            options.insert("api_key".to_string(), "SER_API_KEY".to_string());
            options.insert("engine".to_string(), "google_ia_overviem".to_string());
            options.insert("q".to_string(),query);

            let client = Client::new(options).unwrap();
            let results = client.search(HashMap::new())
                                .await.map_err(|_| ResponseError::Network)?;
            let respose_ia = results["ia_overview"]["answer"]
                                .as_str()
                                .unwrap_or("Not find a respose of the IA");
                 
            //here integer the logic of find extern
            bot.send_message(msg.chat.id, respose_ia).await;
        }
        Commands::Cumpleaños(mencion) => {
            let data = db.lock();
            //find the name in the json charger [8,9]
            let respuesta = data.get(&mencion)
                .map(|fecha| format!("el cumpleaños de {mencion} es el {fecha}"))
                .unwrap_or_else(|| "Usuarios no encontrado en la base de datos.".to_string());
            bot.send_message(msg.chat.id, respuesta).await;
        }
        Commands::Bloqueo(mencion) => {
            //La logica de bloqueo requiere verificar permisos de admin
            //Teloxide maneja estas peticiones a la API de telegram [4]
            bot.send_message(msg.chat.id, format!("Usuarios {mencion} silenciado por 1 hora.")).await?;
        }
    }
    Ok(())
}
