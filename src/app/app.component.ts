import { Component, OnInit } from "@angular/core";
import { invoke } from "@tauri-apps/api/tauri";
import { FormsModule } from '@angular/forms';
import {StdService} from "./std.service";

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

interface Command {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
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

  commands: Command[] = [];
  stdoutMessages: string[] = [];

  isRunning: boolean = false;
  stopping: boolean = false;
  activeTab: 'settings' | 'logs' = 'settings';
  disabledCommands: string[] = [];

  constructor(private stdService: StdService) {}

  ngOnInit(): void {
    invoke("get_config").then((res) => {
      this.config = res as Config;
    });

    invoke("is_running").then((res) => {
      this.isRunning = res as boolean;
    });

    invoke("get_commands").then((res) => {
        this.commands = res as Command[];
    });

    this.stdService.stdoutData$.subscribe((data) => {
        this.stdoutMessages.push(data);
    });

  }

  toogle() {
    if (this.stopping) {
      return;
    }

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

  changeTab(logs: 'settings' | 'logs'): void {
    this.activeTab = logs;
  }

  updateConfig(): void {
    invoke("save_config", { config: this.config }).then((res) => {
      console.log('updated config');
    });
  }

  updateCommandState(command: Command): void {
    if (this.disabledCommands.includes(command.id)) {
      if (command.enabled) {
        this.disabledCommands = this.disabledCommands.filter((cmd) => cmd !== command.id);
      }
    } else {
      if (!command.enabled) {
        this.disabledCommands.push(command.id);
      }
    }

    invoke("update_disabled_commands", { disabledCommands: this.disabledCommands }).then((res) => {
      console.log('updated command state');
    });
  }


}
