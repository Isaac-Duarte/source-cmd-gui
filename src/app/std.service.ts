import {Injectable, NgZone} from '@angular/core';
import {BehaviorSubject} from "rxjs";
import {appWindow} from "@tauri-apps/api/window";


export interface Log {
    time_stamp: string,
    level: string,
    target: string,
    message: string,
}

@Injectable({
    providedIn: 'root'
})
export class StdService {
    private stdoutData = new BehaviorSubject<Log>({
        time_stamp: '',
        level: '',
        target: '',
        message: '',
    });
    stdoutData$ = this.stdoutData.asObservable();

    constructor(private zone: NgZone) {
        appWindow.listen('stdout_data', (event) => {
            this.zone.run(() => {
                this.stdoutData.next(event.payload as unknown as Log);
            });
        });
    }
}
