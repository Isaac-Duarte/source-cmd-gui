import {Component, OnInit} from '@angular/core';
import {CodeModel} from "@ngstack/code-editor";
import { invoke } from '@tauri-apps/api';

interface Script {
    id?: number;
    name: string;
    code: string;
    trigger: string;
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
                this.currentScript = this.scripts[0];
                this.setModelCode(this.currentScript.code);
            }
        });
    }

    createNewScript(): void {
        const script: Script = {
            name: 'New Script',
            code: '',
            trigger: '',
        };

        invoke('add_script', {script: script}).then((res) => {
            console.log(res);
            this.loadScripts();
            this.selectScript(res as Script);
        });
    }

    selectScript(script: Script): void {
        this.currentScript = script;
        this.setModelCode(script.code);
    }

    deleteScript(): void {
        invoke('delete_script', {id: this.currentScript?.id}).then(() => {
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
            invoke('update_script', {script: this.currentScript}).then(() => {
                this.loadScripts();
            });
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
