import { NgModule } from "@angular/core";
import { BrowserModule } from "@angular/platform-browser";

import { AppComponent } from "./app.component";
import { FormsModule } from "@angular/forms";
import { PythonTabComponent } from './python-tab/python-tab.component';
import {CodeEditorModule} from "@ngstack/code-editor";

@NgModule({
  declarations: [AppComponent, PythonTabComponent],
  imports: [BrowserModule, FormsModule, CodeEditorModule.forRoot()],
  providers: [],
  bootstrap: [AppComponent],
})
export class AppModule {
}
