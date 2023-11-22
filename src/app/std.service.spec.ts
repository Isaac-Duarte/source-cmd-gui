import { TestBed } from '@angular/core/testing';

import { StdService } from './std.service';

describe('StdService', () => {
  let service: StdService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(StdService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
