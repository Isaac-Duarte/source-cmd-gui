import { Component, ElementRef, OnInit, ViewChild } from "@angular/core";
import { invoke } from "@tauri-apps/api/tauri";
import { FormsModule } from '@angular/forms';
import { Log, StdService } from "./std.service";

interface Config {
    file_path: string,
    command_timeout: number,
    owner: String,
    parser: GameParser,
    openai_api_key: string,
    disabled_commands: string[],
    response_direction: string,
}

enum GameParser {

    CounterStrike2 = "Counter Strike 2",
    CounterStrikeSource = "Counter Strike Source",
    Minecraft = "Minecraft",
}

interface Command {
    name: string;
    id: string;
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
        disabled_commands: [],
        response_direction: '',
    };
    

    commands: Command[] = [];
    stdoutMessages: Log[] = [];
    @ViewChild('logContainer') private logContainer!: ElementRef;

    isRunning: boolean = false;
    stopping: boolean = false;
    activeTab: 'settings' | 'logs' | 'python-scripts' = 'settings';

    constructor(private stdService: StdService) {
    }

    ngOnInit(): void {
        invoke("get_commands").then((res) => {
            this.commands = res as Command[];

            invoke("get_config").then((res) => {
                this.config = res as Config;

                this.commands.forEach((command) => {
                    command.enabled = !this.config.disabled_commands?.includes(command.id);
                });
            });

        });


        invoke("is_running").then((res) => {
            this.isRunning = res as boolean;
        });


        this.stdService.stdoutData$.subscribe((data) => {
            if (data.message === '') {
                return;
            }

            this.stdoutMessages.push(data);

            // Only keep first 1000
            if (this.stdoutMessages.length > 1000) {
                this.stdoutMessages = this.stdoutMessages.slice(this.stdoutMessages.length - 1000);
            }

            setTimeout(() => this.scrollToBottom(), 10);
        });

    }

    private scrollToBottom(): void {
        try {
            this.logContainer!.nativeElement.scroll({
                top: this.logContainer!.nativeElement.scrollHeight,
                left: 0,
                behavior: 'smooth'
            });
        } catch (err) {
        }
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

    get toggleButtonStatus(): string {
        if (this.stopping) {
            return "Stopping";
        }

        if (this.isRunning) {
            return "Running";
        }

        return "Stopped";
    }

    isActive(tab: string): boolean {
        return this.activeTab === tab;
    }

    changeTab(logs: 'settings' | 'logs' | 'python-scripts'): void {
        this.activeTab = logs;
        this.scrollToBottom();
    }

    updateConfig(): void {
        invoke("save_config", { config: this.config }).then((res) => {
            console.log('updated config');
        });
    }

    updateCommandState(command: Command): void {
        let disabled_commands = this.config.disabled_commands || [];

        if (disabled_commands.includes(command.id)) {
            if (command.enabled) {
                disabled_commands = disabled_commands.filter((cmd) => cmd !== command.id);
            }
        } else {
            if (!command.enabled) {
                disabled_commands.push(command.id);
            }
        }
        

        this.config.disabled_commands = disabled_commands;
        this.updateConfig();
    }


}
