/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export const enum FrontEndLanguage {
  Html = 0,
  JavaScript = 1,
  Tsx = 2,
  Css = 3,
  TypeScript = 4
}
export interface Pos {
  /** line number starting from 1 */
  line: number
  /** column number starting from 1 */
  column: number
  /** byte offset of the position */
  index: number
}
export interface Range {
  start: Pos
  end: Pos
}
export interface NapiConfig {
  rule: any
  constraints?: any
  language?: FrontEndLanguage
  utils?: any
}
export function parseFiles(paths: string[], callback: (err: null | Error, result: SgRoot) => void): Promise<number>
export interface FindConfig {
  paths: Array<string>
  matcher: NapiConfig
}
export class SgNode {
  range(): Range
  isLeaf(): boolean
  isNamedLeaf(): boolean
  kind(): string
  text(): string
  matches(m: string): boolean
  inside(m: string): boolean
  has(m: string): boolean
  precedes(m: string): boolean
  follows(m: string): boolean
  getMatch(m: string): SgNode | null
  getMultipleMatches(m: string): Array<SgNode>
  children(): Array<SgNode>
  find(matcher: string | number | NapiConfig): SgNode | null
  findAll(matcher: string | number | NapiConfig): Array<SgNode>
  field(name: string): SgNode | null
  parent(): SgNode | null
  child(nth: number): SgNode | null
  ancestors(): Array<SgNode>
  next(): SgNode | null
  nextAll(): Array<SgNode>
  prev(): SgNode | null
  prevAll(): Array<SgNode>
}
export class SgRoot {
  root(): SgNode
  filename(): string
}
export namespace html {
  export function parse(src: string): SgRoot
  export function kind(kindName: string): number
  export function pattern(pattern: string): NapiConfig
  export function findInFiles(config: FindConfig, callback: (err: null | Error, result: SgNode[]) => void): Promise<number>
}
export namespace js {
  export function parse(src: string): SgRoot
  export function kind(kindName: string): number
  export function pattern(pattern: string): NapiConfig
  export function findInFiles(config: FindConfig, callback: (err: null | Error, result: SgNode[]) => void): Promise<number>
}
export namespace jsx {
  export function parse(src: string): SgRoot
  export function kind(kindName: string): number
  export function pattern(pattern: string): NapiConfig
  export function findInFiles(config: FindConfig, callback: (err: null | Error, result: SgNode[]) => void): Promise<number>
}
export namespace ts {
  export function parse(src: string): SgRoot
  export function kind(kindName: string): number
  export function pattern(pattern: string): NapiConfig
  export function findInFiles(config: FindConfig, callback: (err: null | Error, result: SgNode[]) => void): Promise<number>
}
export namespace tsx {
  export function parse(src: string): SgRoot
  export function kind(kindName: string): number
  export function pattern(pattern: string): NapiConfig
  export function findInFiles(config: FindConfig, callback: (err: null | Error, result: SgNode[]) => void): Promise<number>
}
