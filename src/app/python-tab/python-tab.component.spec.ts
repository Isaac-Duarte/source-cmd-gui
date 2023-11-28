import { ComponentFixture, TestBed } from '@angular/core/testing';

import { PythonTabComponent } from './python-tab.component';

describe('PythonTabComponent', () => {
  let component: PythonTabComponent;
  let fixture: ComponentFixture<PythonTabComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ PythonTabComponent ]
    })
    .compileComponents();

    fixture = TestBed.createComponent(PythonTabComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
