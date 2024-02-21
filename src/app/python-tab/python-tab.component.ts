import { Component, OnInit } from '@angular/core';
import { CodeModel } from "@ngstack/code-editor";
import { invoke } from '@tauri-apps/api';

interface Script {
    id?: number;
    name: string;
    code?: string;
    trigger: string;
    enabled: boolean;
}

@Component({
    selector: 'app-python-tab',
    templateUrl: './python-tab.component.html',
    styleUrls: ['./python-tab.component.scss']
})
export class PythonTabComponent implements OnInit {
    theme = 'vs-dark';

    model: CodeModel = {
        language: 'python',
        uri: 'main.py',
        value: '',
    };

    options = {
        contextmenu: true,
        minimap: {
            enabled: true,
        },
    };
    scripts: Script[] = []
    currentScript?: Script;

    loadScripts(): void {
        invoke('get_scripts').then((scripts) => {
            this.scripts = scripts as Script[];

            if (!this.currentScript) {
                this.selectScript(this.scripts[0]);
            }

            if (this.scripts.length == 0) {
                this.setModelCode("");
            }
        });
    }

    createNewScript(): void {
        invoke('add_script', { scriptName: "New Script" }).then((res) => {
            console.log(res);
            this.loadScripts();
            this.selectScript(res as Script);
        });
    }

    selectScript(script: Script): void {
        this.currentScript = script;

        invoke('get_code', { scriptId: this.currentScript?.id }).then((res) => {
            this.setModelCode(res as string);

            if (this.currentScript) {
                this.currentScript.code = res as string;
            }
        });
    }

    deleteScript(): void {
        invoke('delete_script', { id: this.currentScript?.id }).then(() => {
            this.currentScript = undefined;
            this.loadScripts();
        });
    }

    onCodeChange(content: string): void {
        if (this.currentScript) {
            this.currentScript.code = content;
        }
    }

    saveScript(): void {
        if (this.currentScript) {
            this.currentScript.code = this.model.value;

            invoke('update_script', { script: this.currentScript }).then(() => {
                this.loadScripts();
            });

            invoke('save_code', { scriptId: this.currentScript.id, code: this.currentScript.code })
        }
    }

    setModelCode(code: string): void {
        this.model = {
            ...this.model,
            value: code,
        }
    }

    ngOnInit(): void {
        this.loadScripts();
    }
}
