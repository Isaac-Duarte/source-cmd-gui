import {Injectable, NgZone} from '@angular/core';
import {BehaviorSubject} from "rxjs";
import {appWindow} from "@tauri-apps/api/window";

@Injectable({
    providedIn: 'root'
})
export class StdService {
    private stdoutData = new BehaviorSubject<string>('');
    stdoutData$ = this.stdoutData.asObservable();

    constructor(private zone: NgZone) {
        appWindow.listen('stdout_data', (event) => {
            this.zone.run(() => {
                this.stdoutData.next(event.payload as unknown as string);
            });
        });
    }
}
