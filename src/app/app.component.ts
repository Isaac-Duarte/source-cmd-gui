import { Component, OnInit } from "@angular/core";
import { invoke } from "@tauri-apps/api/tauri";
import { FormsModule } from '@angular/forms';

interface Config {
  file_path: string,
  command_timeout: number,
  owner: String,
  parser: GameParser,
  openai_api_key: string,
}

enum GameParser {

  CounterStrike2 = "Counter Strike 2",
  CounterStrikeSource = "Counter Strike Source",
}

@Component({
  selector: "app-root",
  templateUrl: "./app.component.html",
  styleUrls: ["./app.component.scss"],
})
export class AppComponent implements OnInit {
  config: Config = {
    file_path: '',
    command_timeout: 0,
    owner: '',
    parser: GameParser.CounterStrike2,
    openai_api_key: '',
  };

  isRunning: boolean = false;
  stopping: boolean = false;
  activeTab = 'settings';

  ngOnInit(): void {
    invoke("get_config").then((res) => {
      this.config = res as Config;
    });

    invoke("is_running").then((res) => {
      this.isRunning = res as boolean;
    });
  }

  toogle() {
    invoke("is_running").then((res) => {
      this.isRunning = res as boolean;

      if (this.isRunning) {
        this.stopping = true;

        invoke("stop").then((res) => {
          this.isRunning = false;
          this.stopping = false;
        });
      } else {
        invoke("start", { config: this.config }).then((res) => {
          this.isRunning = true;
        });
      }
    });
  }

  get toggleButtonName(): string {
    if (this.stopping) {
      return "Stopping";
    }

    if (this.isRunning) {
      return "Stop";
    }

    return "Start";
  }

  isActive(tab: string): boolean {
    return this.activeTab === tab;
  }
}
