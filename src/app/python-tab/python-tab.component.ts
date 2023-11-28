import {Component} from '@angular/core';
import {CodeModel} from "@ngstack/code-editor";

interface Script {
    id: number;
    name: string;
    code: string;
    trigger: string;
}

@Component({
    selector: 'app-python-tab',
    templateUrl: './python-tab.component.html',
    styleUrls: ['./python-tab.component.scss']
})
export class PythonTabComponent {
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
    scripts: Script[] = this.getScripts();
    currentScript?: Script = this.scripts[0];

    getScripts(): Script[] {
        return [];
    }

    createNewScript(): void {

    }

    selectScript(script: { name: string; code: string }): void {

    }

    editScript(script: { name: string; code: string }): void {

    }

    deleteScript(script: { name: string; code: string }): void {

    }

    onCodeChange(content: string): void {
    }

    saveScript(): void {

    }
}
